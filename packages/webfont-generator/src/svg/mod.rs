mod parse;
mod process;
mod serialize;
#[cfg(test)]
mod tests;
pub(crate) mod types;
mod winding;

use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::sync::Arc;

use parse::parse_svg_glyph;
use process::process_glyph;
pub(crate) use serialize::build_svg_font;
pub(crate) use serialize::rounded_coordinate;
use types::{
    CachedGlyph, CachedProcessedGlyph, GlyphCache, GlyphWorkItem, ParsedGlyph, PreparedSvgFont,
    ProcessedGlyph, SvgOptions,
};

use crate::input::LoadedSvgFile;
use crate::types::{FontType, ResolvedGenerateWebfontsOptions};

struct FinalizePlan {
    normalize: bool,
    fixed_width: bool,
    center_horizontally: bool,
    center_vertically: bool,
    round: f64,
    max_glyph_height: f64,
    font_height: f64,
    font_width: f64,
    ascent: f64,
    descent: f64,
    font_id: String,
    metadata: String,
    optimize_output: bool,
    serialize_path: bool,
    structure_path: bool,
}

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

pub(crate) fn svg_options_from_options(
    options: &ResolvedGenerateWebfontsOptions,
) -> SvgOptions<'_> {
    let svg_format = options
        .format_options
        .as_ref()
        .and_then(|value| value.svg.as_ref());
    let wants_binary = options
        .types
        .iter()
        .any(|font_type| *font_type != FontType::Svg);
    let structure_path = wants_binary;

    SvgOptions {
        ascent: options.ascent,
        center_horizontally: options.center_horizontally,
        center_vertically: options.center_vertically,
        codepoints: &options.codepoints,
        descent: options.descent,
        fixed_width: options.fixed_width,
        font_height: options.font_height,
        font_id: svg_format.and_then(|v| v.font_id.as_deref()),
        font_name: &options.font_name,
        font_style: options.font_style.as_deref(),
        font_weight: options.font_weight.as_deref(),
        ligature: options.ligature,
        metadata: svg_format.and_then(|v| v.metadata.as_deref()),
        normalize: options.normalize,
        optimize_output: options.optimize_output,
        preserve_aspect_ratio: options.preserve_aspect_ratio,
        round: options.round,
        serialize_path: options.types.contains(&FontType::Svg) || !structure_path,
        structure_path,
    }
}

pub(crate) fn prepare_svg_font(
    options: &SvgOptions,
    source_files: &[LoadedSvgFile],
) -> Result<PreparedSvgFont, Error> {
    let glyphs = parse_glyphs(options, source_files)?;
    finalize_glyphs(options, glyphs)
}

/// Parse each SVG file into a [`ParsedGlyph`] (geometry + assigned codepoint/index/name). This
/// is the per-file half of [`prepare_svg_font`]: every glyph is independent and content-derived,
/// which is what lets an incremental rebuild reuse the ones whose source didn't change. The
/// global, set-dependent work (metrics, normalization, glyph processing) lives in
/// [`finalize_glyphs`].
pub(crate) fn parse_glyphs(
    options: &SvgOptions,
    source_files: &[LoadedSvgFile],
) -> Result<Vec<ParsedGlyph>, Error> {
    if source_files.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Expected at least one SVG file for native generation.",
        ));
    }

    let preserve_aspect_ratio = options.preserve_aspect_ratio.unwrap_or(false);
    let parser_options = usvg::Options::default();

    let mut work_items = Vec::with_capacity(source_files.len());
    for (index, source_file) in source_files.iter().enumerate() {
        let name = &source_file.glyph_name;
        let codepoint = options
            .codepoints
            .get(name.as_str())
            .copied()
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Missing resolved codepoint for glyph '{name}'."),
                )
            })?;

        work_items.push(GlyphWorkItem {
            codepoint,
            index,
            name,
            source_file,
        });
    }

    let mut glyphs = work_items
        .par_iter()
        .map(|item| parse_svg_glyph(item, preserve_aspect_ratio, &parser_options))
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
    glyphs.sort_by_key(|glyph| glyph.index);
    Ok(glyphs)
}

/// Turn parsed glyphs into a [`PreparedSvgFont`]: compute the set-wide metrics (tallest/widest,
/// font height/width, ascent/descent) and run per-glyph processing (normalize, center, round,
/// optimize). This is the global half of [`prepare_svg_font`] — it depends on the whole glyph
/// set, so an incremental rebuild must re-run it even when only one glyph changed.
pub(crate) fn finalize_glyphs(
    options: &SvgOptions,
    glyphs: Vec<ParsedGlyph>,
) -> Result<PreparedSvgFont, Error> {
    let plan = finalize_plan(options, &glyphs, |glyph| glyph.height, |glyph| glyph.width);

    let mut processed_glyphs = glyphs
        .into_par_iter()
        .map(|glyph| process_glyph_with_plan(glyph, &plan))
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
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

fn finalize_plan<T>(
    options: &SvgOptions,
    glyphs: &[T],
    height: impl Fn(&T) -> f64,
    width: impl Fn(&T) -> f64,
) -> FinalizePlan {
    let normalize = options.normalize;
    let max_glyph_height = glyphs
        .iter()
        .fold(0.0_f64, |current, glyph| current.max(height(glyph)));
    let max_glyph_width = glyphs
        .iter()
        .fold(0.0_f64, |current, glyph| current.max(width(glyph)));
    let font_height = options.font_height.unwrap_or(max_glyph_height.max(1.0));
    let descent = options.descent.unwrap_or(0.0);
    let mut font_width = if max_glyph_height > 0.0 {
        max_glyph_width
    } else {
        max_glyph_width.max(1.0)
    };
    if normalize {
        font_width = glyphs
            .iter()
            .map(|glyph| {
                if height(glyph) > 0.0 {
                    (font_height / height(glyph)) * width(glyph)
                } else {
                    width(glyph)
                }
            })
            .fold(0.0_f64, f64::max);
    } else if options.font_height.is_some() && max_glyph_height > 0.0 {
        font_width *= font_height / max_glyph_height;
    }

    FinalizePlan {
        normalize,
        fixed_width: options.fixed_width.unwrap_or(false),
        center_horizontally: options.center_horizontally.unwrap_or(false),
        center_vertically: options.center_vertically.unwrap_or(false),
        round: options.round.unwrap_or(10e12),
        max_glyph_height,
        font_height,
        font_width,
        ascent: options.ascent.unwrap_or(font_height - descent),
        descent,
        font_id: options.font_id.unwrap_or(options.font_name).to_owned(),
        metadata: options.metadata.unwrap_or_default().to_owned(),
        optimize_output: options.optimize_output.unwrap_or(false),
        serialize_path: options.serialize_path,
        structure_path: options.structure_path,
    }
}

fn process_glyph_with_plan(
    glyph: ParsedGlyph,
    plan: &FinalizePlan,
) -> Result<ProcessedGlyph, Error> {
    process_glyph(
        glyph,
        plan.normalize,
        plan.fixed_width,
        plan.center_horizontally,
        plan.center_vertically,
        plan.round,
        plan.max_glyph_height,
        plan.font_height,
        plan.font_width,
        plan.descent,
        plan.optimize_output,
        plan.serialize_path,
        plan.structure_path,
    )
}

/// Like [`prepare_svg_font`], but reuses cached glyph geometry instead of re-parsing. A file
/// present in `cache` is treated as unchanged and reused; anything else is parsed and cached.
/// Drives both the first (incremental) build — empty cache, so every glyph is parsed and stored —
/// and a later rebuild, where the caller (`regenerate`) has evicted the paths it knows changed.
/// The global [`finalize_glyphs`] pass still runs over the whole set, so the output is
/// byte-identical to [`prepare_svg_font`] for the same inputs.
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

fn processed_glyph_cache_signature(plan: &FinalizePlan) -> [u8; 16] {
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
