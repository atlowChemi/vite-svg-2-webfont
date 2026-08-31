use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::sync::Arc;

use rayon::prelude::*;

use super::parse::parse_svg_glyph;
use super::types::{
    CachedGlyph, CachedProcessedGlyph, GlyphCache, GlyphWorkItem, ParsedGlyph, PreparedSvgFont,
    ProcessedGlyph, SvgOptions,
};
use super::{finalize_plan, process_glyph_with_plan};
use crate::input::LoadedSvgFile;

enum IncrementalGlyph {
    Fresh(ParsedGlyph),
    Cached {
        codepoint: u32,
        index: usize,
        name: String,
        glyph: Arc<CachedGlyph>,
    },
}

impl IncrementalGlyph {
    fn dimensions(&self) -> (f64, f64) {
        match self {
            Self::Fresh(glyph) => (glyph.height, glyph.width),
            Self::Cached { glyph, .. } => (glyph.height, glyph.width),
        }
    }

    fn index(&self) -> usize {
        match self {
            Self::Fresh(glyph) => glyph.index,
            Self::Cached { index, .. } => *index,
        }
    }

    fn into_parsed(self) -> ParsedGlyph {
        match self {
            Self::Fresh(glyph) => glyph,
            Self::Cached {
                codepoint,
                index,
                name,
                glyph,
            } => ParsedGlyph {
                codepoint,
                height: glyph.height,
                index,
                name,
                paths: glyph.paths.clone(),
                width: glyph.width,
            },
        }
    }
}

/// Like [`super::prepare_svg_font`], but reuses cached glyph geometry instead of re-parsing. A file
/// present in `cache` is treated as unchanged and reused; anything else is parsed and cached.
/// Drives both the first (incremental) build — empty cache, so every glyph is parsed and stored —
/// and a later rebuild, where the caller (`regenerate`) has evicted the paths it knows changed.
/// The global [`super::finalize_glyphs`] pass still runs over the whole set, so the output is
/// byte-identical to [`super::prepare_svg_font`] for the same inputs.
pub(crate) fn prepare_svg_font_incremental(
    options: &SvgOptions,
    source_files: &[LoadedSvgFile],
    cache: &mut GlyphCache,
) -> Result<PreparedSvgFont, Error> {
    let glyphs = parse_glyphs_incremental(options, source_files, cache)?;
    finalize_glyphs_incremental(options, glyphs, source_files, cache)
}

pub(crate) fn source_content_hash(contents: &str) -> [u8; 16] {
    md5::compute(contents.as_bytes()).0
}

fn parse_glyphs_incremental(
    options: &SvgOptions,
    source_files: &[LoadedSvgFile],
    cache: &mut GlyphCache,
) -> Result<Vec<IncrementalGlyph>, Error> {
    if source_files.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Expected at least one SVG file for native generation.",
        ));
    }

    let preserve_aspect_ratio = options.preserve_aspect_ratio.unwrap_or(false);

    // Forget cache entries for files no longer in the set.
    let current: HashSet<&str> = source_files.iter().map(|file| file.path.as_str()).collect();
    cache
        .entries
        .retain(|path, _| current.contains(path.as_str()));
    cache
        .content_hashes
        .retain(|path, _| current.contains(path.as_str()));
    cache
        .processed_entries
        .retain(|path, _| current.contains(path.as_str()));

    // Rehydrate path entries from content-addressed geometry where possible. This handles added
    // files whose SVG bytes match an existing glyph (including rename-like remove/add events).
    for source_file in source_files {
        if cache.entries.contains_key(&source_file.path) {
            continue;
        }
        let hash = source_content_hash(&source_file.contents);
        if let Some(cached) = cache.by_content_hash.get(&hash) {
            cache
                .entries
                .insert(source_file.path.clone(), cached.clone());
            cache.content_hashes.insert(source_file.path.clone(), hash);
        }
    }

    // Resolve each file's codepoint up front.
    let mut codepoints = Vec::with_capacity(source_files.len());
    for source_file in source_files {
        let codepoint = options
            .codepoints
            .get(source_file.glyph_name.as_str())
            .copied()
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Missing resolved codepoint for glyph '{}'.",
                        source_file.glyph_name
                    ),
                )
            })?;
        codepoints.push(codepoint);
    }

    // Parse (in parallel) only files not already cached — a present entry is reused as-is.
    let parser_options = usvg::Options::default();
    let parsed: Vec<(usize, ParsedGlyph)> = source_files
        .par_iter()
        .enumerate()
        .filter(|(_, source_file)| !cache.entries.contains_key(&source_file.path))
        .map(|(index, source_file)| {
            let work = GlyphWorkItem {
                codepoint: codepoints[index],
                index,
                name: &source_file.glyph_name,
                source_file,
            };
            parse_svg_glyph(&work, preserve_aspect_ratio, &parser_options)
                .map(|glyph| (index, glyph))
        })
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;

    #[cfg(test)]
    {
        cache.parse_count += parsed.len();
    }

    // Cache the freshly-parsed geometry.
    for (index, glyph) in &parsed {
        let source_file = &source_files[*index];
        let hash = source_content_hash(&source_file.contents);
        let cached = Arc::new(CachedGlyph {
            height: glyph.height,
            paths: glyph.paths.clone(),
            width: glyph.width,
        });
        cache.by_content_hash.insert(hash, cached.clone());
        cache.content_hashes.insert(source_file.path.clone(), hash);
        cache.entries.insert(source_file.path.clone(), cached);
    }

    let active_hashes: HashSet<[u8; 16]> = cache.content_hashes.values().copied().collect();
    cache
        .by_content_hash
        .retain(|hash, _| active_hashes.contains(hash));

    // Assemble the full set without cloning cached paths until processing actually needs them.
    let mut freshly_parsed: HashMap<usize, ParsedGlyph> = parsed.into_iter().collect();
    let mut glyphs = Vec::with_capacity(source_files.len());
    for (index, source_file) in source_files.iter().enumerate() {
        let glyph = match freshly_parsed.remove(&index) {
            Some(glyph) => IncrementalGlyph::Fresh(glyph),
            None => {
                let cached = cache
                    .entries
                    .get(&source_file.path)
                    .expect("an unchanged file must have a cache entry");
                IncrementalGlyph::Cached {
                    codepoint: codepoints[index],
                    index,
                    name: source_file.glyph_name.clone(),
                    glyph: Arc::clone(cached),
                }
            }
        };
        glyphs.push(glyph);
    }
    glyphs.sort_by_key(IncrementalGlyph::index);
    Ok(glyphs)
}

fn processed_glyph_cache_signature(plan: &super::FinalizePlan) -> [u8; 16] {
    let mut bytes = Vec::with_capacity(8 * 5 + 7);
    bytes.extend_from_slice(&[
        plan.normalize as u8,
        plan.fixed_width as u8,
        plan.center_horizontally as u8,
        plan.center_vertically as u8,
        plan.optimize_output as u8,
        plan.serialize_path as u8,
        plan.structure_path as u8,
    ]);
    for value in [
        plan.round,
        plan.max_glyph_height,
        plan.font_height,
        plan.font_width,
        plan.descent,
    ] {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    md5::compute(bytes).0
}

fn finalize_glyphs_incremental(
    options: &SvgOptions,
    glyphs: Vec<IncrementalGlyph>,
    source_files: &[LoadedSvgFile],
    cache: &mut GlyphCache,
) -> Result<PreparedSvgFont, Error> {
    let plan = finalize_plan(
        options,
        &glyphs,
        |glyph| glyph.dimensions().0,
        |glyph| glyph.dimensions().1,
    );
    let signature = processed_glyph_cache_signature(&plan);
    if cache.processed_signature != Some(signature) {
        cache.processed_entries.clear();
        cache.processed_signature = Some(signature);
    }

    let processed: Vec<(usize, ProcessedGlyph, CachedProcessedGlyph)> = glyphs
        .into_par_iter()
        .enumerate()
        .filter(|(_, glyph)| {
            !cache
                .processed_entries
                .contains_key(&source_files[glyph.index()].path)
        })
        .map(|(_, glyph)| {
            let path_index = glyph.index();
            process_glyph_with_plan(glyph.into_parsed(), &plan).map(|glyph| {
                let cached = CachedProcessedGlyph {
                    height: glyph.height,
                    path_data: glyph.path_data.clone(),
                    ttf_path: glyph.ttf_path.clone(),
                    ttf_path_hash: glyph.ttf_path_hash,
                    width: glyph.width,
                };
                (path_index, glyph, cached)
            })
        })
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;

    #[cfg(test)]
    {
        cache.process_count += processed.len();
    }

    let mut freshly_processed = HashMap::with_capacity(processed.len());
    for (index, glyph, cached) in processed {
        cache
            .processed_entries
            .insert(source_files[index].path.clone(), cached);
        freshly_processed.insert(index, glyph);
    }

    let mut processed_glyphs = Vec::with_capacity(source_files.len());
    for (index, source_file) in source_files.iter().enumerate() {
        let glyph = match freshly_processed.remove(&index) {
            Some(glyph) => glyph,
            None => {
                let cached = cache
                    .processed_entries
                    .get(&source_file.path)
                    .expect("an unchanged file must have a processed cache entry");
                let codepoint = glyphs_codepoint(options, source_file)?;
                ProcessedGlyph {
                    codepoint,
                    height: cached.height,
                    index,
                    name: source_file.glyph_name.clone(),
                    path_data: cached.path_data.clone(),
                    ttf_path: cached.ttf_path.clone(),
                    ttf_path_hash: cached.ttf_path_hash,
                    width: cached.width,
                }
            }
        };
        processed_glyphs.push(glyph);
    }
    processed_glyphs.sort_by_key(|glyph| glyph.index);

    Ok(PreparedSvgFont {
        ascent: plan.ascent,
        descent: plan.descent,
        font_height: plan.font_height,
        font_id: plan.font_id,
        font_width: plan.font_width,
        metadata: plan.metadata,
        processed_glyphs,
    })
}

fn glyphs_codepoint(options: &SvgOptions, source_file: &LoadedSvgFile) -> Result<u32, Error> {
    options
        .codepoints
        .get(source_file.glyph_name.as_str())
        .copied()
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Missing resolved codepoint for glyph '{}'.",
                    source_file.glyph_name
                ),
            )
        })
}
