use std::io::Result;

use super::sources::{MembershipChange, change_membership};
use crate::result::RegenerateError;
use crate::{GenerateWebfontsResult, GlyphChange, RegenerationFiles, VariantFileSet};

impl GenerateWebfontsResult {
    fn membership_for_change(
        &self,
        path: &str,
        change: MembershipChange<'_>,
    ) -> Result<RegenerationFiles> {
        let names = self.options.variants.as_ref().map(|family| {
            family
                .variants
                .iter()
                .map(|variant| variant.name.clone())
                .collect::<Vec<_>>()
        });
        let mut sets = match &self.options.variants {
            Some(family) => family
                .variants
                .iter()
                .map(|variant| variant.source_ids.clone())
                .collect(),
            None => vec![
                self.source_files
                    .iter()
                    .map(|source| source.id.clone())
                    .collect(),
            ],
        };
        change_membership(&mut sets, names.as_deref(), path, change)?;
        Ok(match names {
            Some(names) => RegenerationFiles::Variants(
                names
                    .into_iter()
                    .zip(sets)
                    .map(|(variant, files)| VariantFileSet { variant, files })
                    .collect(),
            ),
            None => RegenerationFiles::Single(sets.remove(0)),
        })
    }

    /// Append a new file and regenerate. Families require an existing target variant.
    /// `name` overrides the file stem. Requires incremental generation and inherits
    /// [`Self::regenerate`]'s validation, callback restrictions and write/failure semantics.
    pub fn add(&mut self, path: String, name: Option<String>, variant: Option<&str>) -> Result<()> {
        let files = self.membership_for_change(&path, MembershipChange::Add(variant))?;
        self.regenerate(&files, &[(path, GlyphChange::Added { name })])
    }

    /// Re-read an existing file in every consuming design, preserving its position.
    /// An absent `name` preserves the glyph name. Inherits [`Self::regenerate`]'s semantics.
    pub fn update(&mut self, path: String, name: Option<String>) -> Result<()> {
        let files = self.membership_for_change(&path, MembershipChange::Update)?;
        self.regenerate(&files, &[(path, GlyphChange::Changed { name })])
    }

    /// Remove an existing file from every design without deleting it from disk.
    /// Rejects empty designs. Inherits [`Self::regenerate`]'s semantics.
    pub fn remove(&mut self, path: String) -> Result<()> {
        let files = self.membership_for_change(&path, MembershipChange::Remove)?;
        self.regenerate(&files, &[(path, GlyphChange::Removed)])
    }

    /// Consuming async equivalent of [`Self::add`], with [`Self::regenerate_async`]'s recovery semantics.
    pub async fn add_async(
        self,
        path: String,
        name: Option<String>,
        variant: Option<String>,
    ) -> std::result::Result<Self, RegenerateError> {
        let files =
            match self.membership_for_change(&path, MembershipChange::Add(variant.as_deref())) {
                Ok(files) => files,
                Err(error) => return Err(RegenerateError::new(Some(self), error)),
            };
        self.regenerate_async(files, vec![(path, GlyphChange::Added { name })])
            .await
    }

    /// Consuming async equivalent of [`Self::update`], with [`Self::regenerate_async`]'s recovery semantics.
    pub async fn update_async(
        self,
        path: String,
        name: Option<String>,
    ) -> std::result::Result<Self, RegenerateError> {
        let files = match self.membership_for_change(&path, MembershipChange::Update) {
            Ok(files) => files,
            Err(error) => return Err(RegenerateError::new(Some(self), error)),
        };
        self.regenerate_async(files, vec![(path, GlyphChange::Changed { name })])
            .await
    }

    /// Consuming async equivalent of [`Self::remove`], with [`Self::regenerate_async`]'s recovery semantics.
    pub async fn remove_async(self, path: String) -> std::result::Result<Self, RegenerateError> {
        let files = match self.membership_for_change(&path, MembershipChange::Remove) {
            Ok(files) => files,
            Err(error) => return Err(RegenerateError::new(Some(self), error)),
        };
        self.regenerate_async(files, vec![(path, GlyphChange::Removed)])
            .await
    }

    /// Snapshot-preserving async addition for adapters; inherits [`Self::regenerate_snapshot_async`]'s lineage and cancellation rules.
    pub async fn add_snapshot_async(
        &self,
        path: String,
        name: Option<String>,
        variant: Option<String>,
    ) -> Result<Self> {
        let files = self.membership_for_change(&path, MembershipChange::Add(variant.as_deref()))?;
        self.regenerate_snapshot_async(files, Some(vec![(path, GlyphChange::Added { name })]))
            .await
    }

    /// Snapshot-preserving async update for adapters; inherits [`Self::regenerate_snapshot_async`]'s lineage and cancellation rules.
    pub async fn update_snapshot_async(&self, path: String, name: Option<String>) -> Result<Self> {
        let files = self.membership_for_change(&path, MembershipChange::Update)?;
        self.regenerate_snapshot_async(files, Some(vec![(path, GlyphChange::Changed { name })]))
            .await
    }

    /// Snapshot-preserving async removal for adapters; inherits [`Self::regenerate_snapshot_async`]'s lineage and cancellation rules.
    pub async fn remove_snapshot_async(&self, path: String) -> Result<Self> {
        let files = self.membership_for_change(&path, MembershipChange::Remove)?;
        self.regenerate_snapshot_async(files, Some(vec![(path, GlyphChange::Removed)]))
            .await
    }
}
