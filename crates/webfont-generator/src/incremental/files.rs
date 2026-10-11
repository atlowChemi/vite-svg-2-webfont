//! File acquisition and legacy hint semantics, before the shared content-update boundary.
use std::cell::LazyCell;
use std::collections::HashMap;
use std::path::Path;

use super::RegenerationState;
use super::sources::{SourceUpdate, refresh_source_cache, replace_source_cache};
use crate::GlyphChange;
use crate::input::LoadedSvgSource;
use crate::svg::source_content_hash;

pub(super) fn load_source_updates(
    previous: &[LoadedSvgSource],
    changes: &[(String, GlyphChange)],
    state: &mut RegenerationState,
) -> std::io::Result<(Vec<SourceUpdate>, bool)> {
    // Content-only no-op hints need neither source staging nor a name index.
    let mut names = LazyCell::new(|| {
        let mut names = HashMap::new();
        for source in previous {
            names
                .entry(source.id.as_str())
                .or_insert(source.glyph_name.as_str());
        }
        names
    });
    let mut updates = Vec::with_capacity(changes.len());
    let mut changed = false;
    for (path, change) in changes {
        match change {
            GlyphChange::Removed => {
                state.caches_dirty = true;
                changed |= names.remove(path.as_str()).is_some();
                replace_source_cache(&mut state.glyph_cache, path, None);
                updates.push(SourceUpdate::Remove(path.clone()));
            }
            GlyphChange::Added { name } | GlyphChange::Changed { name } => {
                let contents = std::fs::read_to_string(path)?;
                let hash = source_content_hash(&contents);
                let added = matches!(change, GlyphChange::Added { .. });
                let content_changed = if added {
                    replace_source_cache(&mut state.glyph_cache, path, Some(hash));
                    true
                } else {
                    refresh_source_cache(&mut state.glyph_cache, path, hash)
                };
                state.caches_dirty |= content_changed;
                let renamed = name
                    .as_deref()
                    .is_some_and(|name| names.get(path.as_str()).copied() != Some(name));
                if content_changed || renamed {
                    changed = true;
                    let glyph_name = name
                        .as_deref()
                        .or_else(|| names.get(path.as_str()).copied())
                        .unwrap_or_else(|| {
                            Path::new(path)
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .unwrap_or_default()
                        });
                    names.insert(path.as_str(), glyph_name);
                    updates.push(SourceUpdate::Set(LoadedSvgSource {
                        id: path.clone(),
                        contents: contents.into(),
                        glyph_name: glyph_name.to_owned(),
                    }));
                }
            }
        }
    }
    Ok((updates, changed))
}
