use std::collections::{HashMap, HashSet};
#[cfg(feature = "async")]
use std::io::Error;
use std::io::Result;
use std::sync::OnceLock;

use super::options::{invalid, validate_name};
use super::{GenerateWebfontsFromSourcesResult, SvgSource, build};
use crate::incremental::sources::{
    MembershipChange, SourceUpdate, change_membership, ordered_sources, same_sources,
    updated_sources,
};
use crate::input::{LoadedSvgSource, validate_glyph_names};
use crate::rendering::css::{TemplateDependencies, template_dependencies};

/// A changed source only; unchanged SVG text stays retained by the result.
/// Deserializes objects tagged with `type`: `added`, `changed`, or `removed`.
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum SourceChange {
    /// Introduce a previously unknown ID with its name and SVG text.
    Added(SvgSource),
    /// Replace an existing source; an absent name preserves its old name.
    Changed {
        /// Existing opaque ID.
        id: String,
        /// Replacement SVG text.
        svg: String,
        /// Optional replacement glyph name.
        name: Option<String>,
    },
    /// Delete an existing ID from all designs.
    Removed {
        /// Existing opaque ID.
        id: String,
    },
}

/// Complete ordered ID membership for a named variant.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariantSourceSet {
    /// Existing variant name; variant configuration cannot change during updates.
    pub name: String,
    /// Complete ordered IDs in this design.
    pub ids: Vec<String>,
}

/// Complete membership after applying a source change batch.
/// Deserializes either `{ "sources": [...] }` or `{ "variants": [...] }`.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum SourceMembership {
    /// Ordinary generation: ordered source IDs.
    Sources(Vec<String>),
    /// Variant generation: all designs, in original order.
    Variants(Vec<VariantSourceSet>),
}

/// An async update failure with the previous usable result, except on worker panic/cancellation.
#[cfg(feature = "async")]
pub struct SourceRegenerateError {
    error: Error,
    result: Option<Box<GenerateWebfontsFromSourcesResult>>,
}

#[cfg(feature = "async")]
impl SourceRegenerateError {
    pub(super) fn worker(error: tokio::task::JoinError) -> Self {
        Self {
            error: Error::other(format!("Source update task failed: {error}")),
            result: None,
        }
    }
    /// Inspect the underlying validation, generation, or worker error.
    pub fn error(&self) -> &Error {
        &self.error
    }
    /// Recover the previous result. Worker join failures cannot recover moved state.
    pub fn into_result(self) -> Option<GenerateWebfontsFromSourcesResult> {
        self.result.map(|result| *result)
    }
    /// Recover both the error and the previous result.
    pub fn into_parts(self) -> (Error, Option<GenerateWebfontsFromSourcesResult>) {
        (self.error, self.result.map(|result| *result))
    }
}

#[cfg(feature = "async")]
impl std::fmt::Debug for SourceRegenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceRegenerateError")
            .field("error", &self.error)
            .field("recoverable", &self.result.is_some())
            .finish()
    }
}
#[cfg(feature = "async")]
impl std::fmt::Display for SourceRegenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
#[cfg(feature = "async")]
impl std::error::Error for SourceRegenerateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl GenerateWebfontsFromSourcesResult {
    fn membership_for_change(
        &self,
        id: &str,
        change: MembershipChange<'_>,
    ) -> Result<SourceMembership> {
        let names = self.options.variants.as_ref().map(|family| {
            family
                .variants
                .iter()
                .map(|variant| variant.name.clone())
                .collect::<Vec<_>>()
        });
        let mut sets = self
            .options
            .sources
            .iter()
            .map(|sources| sources.iter().map(|source| source.id.clone()).collect())
            .collect::<Vec<_>>();
        change_membership(&mut sets, names.as_deref(), id, change)?;
        Ok(match names {
            Some(names) => SourceMembership::Variants(
                names
                    .into_iter()
                    .zip(sets)
                    .map(|(name, ids)| VariantSourceSet { name, ids })
                    .collect(),
            ),
            None => SourceMembership::Sources(sets.remove(0)),
        })
    }

    /// Append a new source and regenerate. Families require an existing target variant.
    /// Requires incremental generation. Inherits [`Self::regenerate_sync`]'s rollback semantics.
    pub fn add_sync(&mut self, source: SvgSource, variant: Option<&str>) -> Result<()> {
        let membership = self.membership_for_change(&source.id, MembershipChange::Add(variant))?;
        self.regenerate_sync(membership, vec![SourceChange::Added(source)])
    }

    /// Replace an existing source in every consuming design, preserving its position.
    /// An absent name preserves the old name. Inherits [`Self::regenerate_sync`]'s rollback semantics.
    pub fn update_sync(&mut self, id: String, svg: String, name: Option<String>) -> Result<()> {
        let membership = self.membership_for_change(&id, MembershipChange::Update)?;
        self.regenerate_sync(membership, vec![SourceChange::Changed { id, svg, name }])
    }

    /// Remove an existing ID from every design. Rejects empty designs.
    /// Inherits [`Self::regenerate_sync`]'s rollback semantics.
    pub fn remove_sync(&mut self, id: String) -> Result<()> {
        let membership = self.membership_for_change(&id, MembershipChange::Remove)?;
        self.regenerate_sync(membership, vec![SourceChange::Removed { id }])
    }

    /// Consuming async equivalent of [`Self::add_sync`], with [`Self::regenerate`]'s recovery semantics.
    #[cfg(feature = "async")]
    pub async fn add(
        self,
        source: SvgSource,
        variant: Option<String>,
    ) -> std::result::Result<Self, SourceRegenerateError> {
        let membership = match self
            .membership_for_change(&source.id, MembershipChange::Add(variant.as_deref()))
        {
            Ok(membership) => membership,
            Err(error) => {
                return Err(SourceRegenerateError {
                    error,
                    result: Some(Box::new(self)),
                });
            }
        };
        self.regenerate(membership, vec![SourceChange::Added(source)])
            .await
    }

    /// Consuming async equivalent of [`Self::update_sync`], with [`Self::regenerate`]'s recovery semantics.
    #[cfg(feature = "async")]
    pub async fn update(
        self,
        id: String,
        svg: String,
        name: Option<String>,
    ) -> std::result::Result<Self, SourceRegenerateError> {
        let membership = match self.membership_for_change(&id, MembershipChange::Update) {
            Ok(membership) => membership,
            Err(error) => {
                return Err(SourceRegenerateError {
                    error,
                    result: Some(Box::new(self)),
                });
            }
        };
        self.regenerate(membership, vec![SourceChange::Changed { id, svg, name }])
            .await
    }

    /// Consuming async equivalent of [`Self::remove_sync`], with [`Self::regenerate`]'s recovery semantics.
    #[cfg(feature = "async")]
    pub async fn remove(self, id: String) -> std::result::Result<Self, SourceRegenerateError> {
        let membership = match self.membership_for_change(&id, MembershipChange::Remove) {
            Ok(membership) => membership,
            Err(error) => {
                return Err(SourceRegenerateError {
                    error,
                    result: Some(Box::new(self)),
                });
            }
        };
        self.regenerate(membership, vec![SourceChange::Removed { id }])
            .await
    }

    /// Apply a changed-source batch and complete ordered membership synchronously.
    /// Requires `incremental: Some(true)`. On error, buffers, sources and renders remain usable;
    /// a failed build discards partially mutated geometry caches so a retry starts safely.
    pub fn regenerate_sync(
        &mut self,
        membership: SourceMembership,
        changes: Vec<SourceChange>,
    ) -> Result<()> {
        if self.caches.is_none() {
            return Err(invalid("Source updates require incremental generation"));
        }
        let memberships = match (&self.options.variants, membership) {
            (None, SourceMembership::Sources(ids)) => vec![ids],
            (Some(variants), SourceMembership::Variants(sets)) => {
                if sets.len() != variants.variants.len()
                    || sets
                        .iter()
                        .zip(&variants.variants)
                        .any(|(set, variant)| set.name != variant.name)
                {
                    return Err(invalid("Supply every variant in its original order"));
                }
                sets.into_iter().map(|set| set.ids).collect()
            }
            _ => {
                return Err(invalid(
                    "Source membership does not match the generation mode",
                ));
            }
        };
        if changes.is_empty()
            && memberships
                .iter()
                .zip(&self.options.sources)
                .all(|(ids, sources)| {
                    ids.iter()
                        .map(String::as_str)
                        .eq(sources.iter().map(|source| source.id.as_str()))
                })
        {
            return Ok(());
        }
        let mut staged = self.options.clone();
        if let Some(variants) = &mut staged.variants {
            for (variant, ids) in variants.variants.iter_mut().zip(&memberships) {
                variant.source_ids = ids.clone();
            }
        }
        let previous = staged
            .sources
            .iter()
            .flatten()
            .map(|source| (source.id.clone(), source.clone()))
            .collect::<HashMap<_, _>>();
        let mut changed_ids = HashSet::new();
        let mut supplied_ids = HashSet::new();
        let mut updates = Vec::with_capacity(changes.len());
        for change in changes {
            let id = match &change {
                SourceChange::Added(source) => &source.id,
                SourceChange::Changed { id, .. } | SourceChange::Removed { id } => id,
            };
            if id.is_empty() || !changed_ids.insert(id.clone()) {
                return Err(invalid("Change IDs must be nonempty and unique"));
            }
            match change {
                SourceChange::Added(source) => {
                    if previous.contains_key(&source.id) {
                        return Err(invalid(format!("Source {:?} already exists", source.id)));
                    }
                    supplied_ids.insert(source.id.clone());
                    updates.push(SourceUpdate::Set(LoadedSvgSource {
                        id: source.id,
                        glyph_name: source.name,
                        contents: source.svg.into(),
                    }));
                }
                SourceChange::Changed { id, svg, name } => {
                    let source = previous
                        .get(&id)
                        .ok_or_else(|| invalid(format!("Unknown source {id:?}")))?;
                    supplied_ids.insert(id.clone());
                    updates.push(SourceUpdate::Set(LoadedSvgSource {
                        id,
                        contents: svg.into(),
                        glyph_name: name.unwrap_or_else(|| source.glyph_name.clone()),
                    }));
                }
                SourceChange::Removed { id } => {
                    if !previous.contains_key(&id) {
                        return Err(invalid(format!("Unknown source {id:?}")));
                    }
                    updates.push(SourceUpdate::Remove(id));
                }
            }
        }
        let previous = previous.into_values().collect::<Vec<_>>();
        let definitions = updated_sources(&previous, &updates);
        let mut used = HashSet::new();
        staged.sources = memberships
            .into_iter()
            .map(|ids| {
                if ids.is_empty() {
                    return Err(invalid("Each design must contain at least one source"));
                }
                let mut seen = HashSet::new();
                for id in &ids {
                    if !seen.insert(id.clone()) {
                        return Err(invalid(format!("Duplicate source ID {id:?}")));
                    }
                    used.insert(id.clone());
                }
                let sources = ordered_sources(&definitions, &ids)
                    .map_err(|id| invalid(format!("Unknown membership ID {id:?}")))?;
                for source in &sources {
                    validate_name(&source.glyph_name)?;
                }
                validate_glyph_names(&sources)?;
                Ok(sources)
            })
            .collect::<Result<Vec<_>>>()?;
        if supplied_ids.iter().any(|id| !used.contains(id)) {
            return Err(invalid(
                "Added or changed sources must appear in final membership",
            ));
        }
        let same = staged
            .sources
            .iter()
            .zip(&self.options.sources)
            .all(|(new, old)| same_sources(old, new));
        if same {
            return Ok(());
        }

        let built = build(&mut staged, self.caches.as_mut(), &updates);
        if let Some(persistent) = self.persistent.as_mut() {
            persistent.caches_reset = built.is_err();
        }
        let fonts = built?;
        let names_unchanged = self
            .options
            .sources
            .iter()
            .flatten()
            .map(|source| &source.glyph_name)
            .eq(staged
                .sources
                .iter()
                .flatten()
                .map(|source| &source.glyph_name));
        let codepoints_unchanged = self.options.common.codepoints == staged.common.codepoints;
        let keep_css = matches!(self.css.get(), Some(Ok(_)))
            && self
                .options
                .css_template
                .as_deref()
                .map(template_dependencies)
                .unwrap_or_else(TemplateDependencies::css_default)
                .can_reuse_css_no_urls(names_unchanged, codepoints_unchanged);
        let html_dependencies = self
            .options
            .html_template
            .as_deref()
            .map(template_dependencies)
            .unwrap_or_else(TemplateDependencies::html_default);
        let keep_html =
            html_dependencies.can_reuse_html(names_unchanged, codepoints_unchanged, keep_css)
                && matches!(self.html.get(), Some(Ok(_)));
        self.options = staged;
        self.fonts = fonts;
        self.shared = OnceLock::new();
        if !keep_css {
            self.css = OnceLock::new();
        }
        if !keep_html {
            self.html = OnceLock::new();
        }
        Ok(())
    }

    /// Apply [`Self::regenerate_sync`] on Tokio's blocking pool, returning the updated result.
    /// Ordinary failures retain the original usable result in [`SourceRegenerateError`].
    #[cfg(feature = "async")]
    pub async fn regenerate(
        mut self,
        membership: SourceMembership,
        changes: Vec<SourceChange>,
    ) -> std::result::Result<Self, SourceRegenerateError> {
        let runtime = match tokio::runtime::Handle::try_current() {
            Ok(runtime) => runtime,
            Err(error) => {
                return Err(SourceRegenerateError {
                    error: Error::other(format!("Source update requires a Tokio runtime: {error}")),
                    result: Some(Box::new(self)),
                });
            }
        };
        runtime
            .spawn_blocking(move || match self.regenerate_sync(membership, changes) {
                Ok(()) => Ok(self),
                Err(error) => Err(SourceRegenerateError {
                    error,
                    result: Some(Box::new(self)),
                }),
            })
            .await
            .map_err(SourceRegenerateError::worker)?
    }
}
