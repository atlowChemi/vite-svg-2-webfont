//! Filesystem-free update boundary. Adapters acquire text and resolve names; this module
//! applies normalized changes, projects ordered membership and reconciles geometry caches.
use std::collections::{HashMap, HashSet};

use crate::input::LoadedSvgSource;
use crate::svg::types::GlyphCache;

pub(crate) enum SourceUpdate {
    Set(LoadedSvgSource),
    Remove(String),
}

pub(crate) enum MembershipChange<'a> {
    Add(Option<&'a str>),
    Update,
    Remove,
}

/// Derive one action's membership without mutating the retained result.
pub(crate) fn change_membership(
    sets: &mut [Vec<String>],
    variants: Option<&[String]>,
    id: &str,
    change: MembershipChange<'_>,
) -> std::io::Result<()> {
    let invalid = |message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message);
    if id.is_empty() {
        return Err(invalid("Source ID/path must not be empty"));
    }
    let exists = sets.iter().any(|set| set.iter().any(|value| value == id));
    match change {
        MembershipChange::Add(variant) => {
            if exists {
                return Err(invalid("Source ID/path already exists"));
            }
            let index = match (variants, variant) {
                (None, None) => 0,
                (Some(names), Some(name)) => names
                    .iter()
                    .position(|value| value == name)
                    .ok_or_else(|| invalid("Unknown target variant"))?,
                (Some(_), None) => {
                    return Err(invalid("Adding to a family requires a target variant"));
                }
                (None, Some(_)) => {
                    return Err(invalid("Ordinary fonts do not accept a target variant"));
                }
            };
            sets[index].push(id.to_owned());
        }
        MembershipChange::Update | MembershipChange::Remove => {
            if !exists {
                return Err(invalid("Unknown source ID/path"));
            }
            if matches!(change, MembershipChange::Remove) {
                for set in sets {
                    set.retain(|value| value != id);
                    if set.is_empty() {
                        return Err(invalid("Each design must contain at least one source"));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Apply changes without touching caches. Memory callers validate the resulting memberships
/// before committing cache changes. File callers retain their legacy failure/dirty-cache policy.
pub(crate) fn updated_sources(
    previous: &[LoadedSvgSource],
    updates: &[SourceUpdate],
) -> HashMap<String, LoadedSvgSource> {
    let mut sources = previous.iter().cloned().map(Some).collect::<Vec<_>>();
    let mut indices = HashMap::new();
    let mut duplicates = HashSet::new();
    for (index, source) in previous.iter().enumerate() {
        if indices.contains_key(source.id.as_str()) {
            duplicates.insert(source.id.as_str());
        } else {
            indices.insert(source.id.as_str(), index);
        }
    }
    for update in updates {
        match update {
            SourceUpdate::Set(source) => {
                if let Some(&index) = indices.get(source.id.as_str()) {
                    sources[index] = Some(source.clone());
                } else {
                    indices.insert(source.id.as_str(), sources.len());
                    sources.push(Some(source.clone()));
                }
            }
            SourceUpdate::Remove(id) => {
                if let Some(index) = indices.remove(id.as_str()) {
                    sources[index] = None;
                    // Legacy file hooks can name repeated occurrences differently: update the
                    // first occurrence, remove every occurrence, then retain the last at ordering.
                    if duplicates.contains(id.as_str()) {
                        for source in &mut sources {
                            if source.as_ref().is_some_and(|source| source.id == *id) {
                                *source = None;
                            }
                        }
                    }
                }
            }
        }
    }
    sources
        .into_iter()
        .flatten()
        .map(|source| (source.id.clone(), source))
        .collect()
}

/// Project one design's authoritative membership. Return the unresolved or repeated ID so
/// each adapter can retain its own public diagnostic wording.
pub(crate) fn ordered_sources(
    definitions: &HashMap<String, LoadedSvgSource>,
    ids: &[String],
) -> Result<Vec<LoadedSvgSource>, String> {
    let mut seen = HashSet::with_capacity(ids.len());
    ids.iter()
        .map(|id| {
            if !seen.insert(id) {
                return Err(id.clone());
            }
            definitions.get(id).cloned().ok_or_else(|| id.clone())
        })
        .collect()
}

pub(crate) fn same_sources(previous: &[LoadedSvgSource], next: &[LoadedSvgSource]) -> bool {
    previous.len() == next.len()
        && previous
            .iter()
            .zip(next)
            .all(|(a, b)| a.id == b.id && a.glyph_name == b.glyph_name && a.contents == b.contents)
}

/// Reconcile one acquired payload with an ID-keyed cache, reusing identical-content geometry.
/// Return whether content state changed. Names and color selection are handled by preparation.
pub(crate) fn refresh_source_cache(cache: &mut GlyphCache, id: &str, hash: [u8; 16]) -> bool {
    if cache.content_hashes.get(id) == Some(&hash) {
        return false;
    }
    replace_source_cache(cache, id, Some(hash));
    true
}

/// Explicit additions in the legacy file API invalidate processed geometry even when their
/// content is unchanged. Removals pass None; ordinary replacements use refresh_source_cache.
pub(crate) fn replace_source_cache(cache: &mut GlyphCache, id: &str, hash: Option<[u8; 16]>) {
    cache.entries.remove(id);
    cache.content_hashes.remove(id);
    cache.processed_entries.remove(id);
    if let Some(hash) = hash
        && let Some(glyph) = cache.by_content_hash.get(&hash)
    {
        cache.entries.insert(id.to_owned(), glyph.clone());
        cache.content_hashes.insert(id.to_owned(), hash);
    }
}

#[cfg(test)]
#[test]
fn normalized_updates_preserve_legacy_duplicate_occurrences_and_ordering() {
    let source = |name: &str| LoadedSvgSource {
        id: "opaque-id".into(),
        glyph_name: name.into(),
        contents: "svg".into(),
    };
    let previous = vec![source("first"), source("last")];
    let ids = vec!["opaque-id".into()];
    let definitions = updated_sources(&previous, &[SourceUpdate::Set(source("renamed-first"))]);
    assert_eq!(
        ordered_sources(&definitions, &ids).unwrap()[0].glyph_name,
        "last"
    );
    let definitions = updated_sources(
        &previous,
        &[
            SourceUpdate::Remove("opaque-id".into()),
            SourceUpdate::Set(source("replacement")),
        ],
    );
    assert_eq!(
        ordered_sources(&definitions, &ids).unwrap()[0].glyph_name,
        "replacement"
    );
    assert!(ordered_sources(&definitions, &["opaque-id".into(), "opaque-id".into()]).is_err());
}
