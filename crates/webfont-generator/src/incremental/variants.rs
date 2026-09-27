use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::sync::Arc;

use crate::input::LoadedSvgFile;
use crate::{GenerateWebfontsResult, GlyphChange, VariantFileSet};

#[cfg(test)]
mod tests;

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidInput, message.into())
}

impl GenerateWebfontsResult {
    pub(super) fn regenerate_variant_files(
        &mut self,
        file_sets: &[VariantFileSet],
        changes: Option<&[(String, GlyphChange)]>,
    ) -> std::io::Result<()> {
        let variants = self
            .options
            .variants
            .as_ref()
            .ok_or_else(|| invalid("Variant file sets require a multi-variant result."))?;
        if !self.options.incremental {
            return Err(invalid("Regeneration requires incremental generation."));
        }
        if self.css_context.is_some() || self.html_context.is_some() {
            return Err(invalid(
                "Regeneration is not supported for cssContext/htmlContext callbacks.",
            ));
        }
        let mut names = HashSet::new();
        let mut current_paths = HashSet::new();
        for set in file_sets {
            if !names.insert(set.variant.as_str()) {
                return Err(invalid(format!("Duplicate variant: {}", set.variant)));
            }
            if !variants.variants.iter().any(|v| v.name == set.variant) {
                return Err(invalid(format!("Unknown variant: {}", set.variant)));
            }
            let paths: HashSet<_> = set.files.iter().map(String::as_str).collect();
            if paths.is_empty() || paths.len() != set.files.len() {
                return Err(invalid(format!(
                    "Variant {} requires nonempty, unique files.",
                    set.variant
                )));
            }
            current_paths.extend(paths);
        }
        if names.len() != variants.variants.len() {
            return Err(invalid(
                "Regeneration requires every configured variant exactly once.",
            ));
        }
        let mut lease = self.take_regeneration_state_lease()?;
        let state = lease.state_mut();
        // Borrow each design's range from the single authoritative source list.
        let mut offset = 0;
        let sources: Vec<_> = variants
            .variants
            .iter()
            .map(|variant| {
                let end = offset + variant.files.len();
                let files = &self.source_files[offset..end];
                offset = end;
                files
            })
            .collect();
        let previous_paths: HashSet<_> =
            self.source_files.iter().map(|s| s.path.as_str()).collect();
        let mut hints = HashMap::new();
        for (path, change) in changes.unwrap_or_default() {
            if hints.insert(path.as_str(), change).is_some() {
                return Err(invalid(format!("Duplicate change path: {path}")));
            }
            let before = previous_paths.contains(path.as_str());
            let after = current_paths.contains(path.as_str());
            let valid = match change {
                GlyphChange::Added { .. } => !before && after,
                GlyphChange::Changed { .. } => before && after,
                GlyphChange::Removed => before && !after,
            };
            if !valid {
                return Err(invalid(format!(
                    "Change does not match family file membership: {path}"
                )));
            }
        }
        let mut options = self.options.as_ref().clone();
        let mut effective = false;
        let mut next_sources = Vec::with_capacity(sources.len());
        // Read each shared path once, so all consumers see the same new contents.
        let mut loaded: HashMap<String, Arc<str>> = HashMap::new();
        for (index, variant) in variants.variants.iter().enumerate() {
            let set = file_sets
                .iter()
                .find(|set| set.variant == variant.name)
                .unwrap();
            let previous: HashMap<_, _> = sources[index]
                .iter()
                .map(|source| (source.path.as_str(), source))
                .collect();
            let next = set
                .files
                .iter()
                .map(|path| {
                    let old = previous.get(path.as_str());
                    let hint = hints.get(path.as_str());
                    let contents = match old {
                        Some(old) if changes.is_some() && hint.is_none() => old.contents.clone(),
                        _ => {
                            if !loaded.contains_key(path) {
                                loaded.insert(path.clone(), std::fs::read_to_string(path)?.into());
                            }
                            loaded[path].clone()
                        }
                    };
                    let name = match hint {
                        Some(GlyphChange::Added { name } | GlyphChange::Changed { name }) => {
                            name.clone()
                        }
                        _ => None,
                    };
                    let glyph_name = name
                        .or_else(|| old.map(|source| source.glyph_name.clone()))
                        .unwrap_or_else(|| {
                            std::path::Path::new(path)
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned()
                        });
                    Ok(LoadedSvgFile {
                        path: path.clone(),
                        contents,
                        glyph_name,
                    })
                })
                .collect::<std::io::Result<Vec<_>>>()?;
            crate::input::validate_glyph_names(&next)?;
            effective |= next.len() != sources[index].len()
                || next.iter().zip(sources[index]).any(|(a, b)| {
                    a.path != b.path || a.glyph_name != b.glyph_name || a.contents != b.contents
                });
            next_sources.push(next);
            options.variants.as_mut().unwrap().variants[index].files = set.files.clone();
        }
        if effective {
            state.caches_dirty = true;
            let cache = state.variant_cache.get_or_insert_with(Default::default);
            let family = crate::prepare_variant_family_cached(
                &mut options,
                next_sources.iter().map(Vec::as_slice).collect(),
                Some(cache),
            )?;
            let fonts = crate::pipeline::build_variant_font_outputs(&options, &family)?;
            let flattened: Vec<_> = next_sources.into_iter().flatten().collect();
            let same_names = flattened
                .iter()
                .map(|source| &source.glyph_name)
                .eq(self.source_files.iter().map(|source| &source.glyph_name));
            let carried = self
                .reusable_render_cache(same_names, options.codepoints == self.options.codepoints);
            self.fonts = fonts;
            self.options = Arc::new(options);
            self.source_files = Arc::new(flattened);
            self.cached = Default::default();
            self.carried_render = carried;
            state.variant_write_pending = self.options.write_files;
            // The cache now matches committed sources. A later write failure retains this
            // usable generation; only failures before this point discard dirty caches.
            state.caches_dirty = false;
        }
        if self.options.write_files && state.variant_write_pending {
            crate::output::write_generate_webfonts_result_sync(self, &mut state.written_outputs)?;
            state.variant_write_pending = false;
        }
        lease.commit();
        Ok(())
    }
}
