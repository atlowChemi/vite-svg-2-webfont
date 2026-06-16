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

use parse::parse_svg_glyph;
use process::process_glyph;
pub(crate) use serialize::build_svg_font;
use types::{CachedGlyph, GlyphCache, GlyphWorkItem, ParsedGlyph, PreparedSvgFont, SvgOptions};

use crate::types::{LoadedSvgFile, ResolvedGenerateWebfontsOptions};

pub(crate) fn svg_options_from_options(
    options: &ResolvedGenerateWebfontsOptions,
) -> SvgOptions<'_> {
    let svg_format = options
        .format_options
        .as_ref()
        .and_then(|value| value.svg.as_ref());

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
        .map(|item| parse_svg_glyph(item, preserve_aspect_ratio))
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
    let normalize = options.normalize;
    let fixed_width = options.fixed_width.unwrap_or(false);
    let center_horizontally = options.center_horizontally.unwrap_or(false);
    let center_vertically = options.center_vertically.unwrap_or(false);
    let ligature = options.ligature;
    let round = options.round.unwrap_or(10e12);

    let max_glyph_height = glyphs
        .iter()
        .fold(0.0_f64, |current, glyph| current.max(glyph.height));
    let max_glyph_width = glyphs
        .iter()
        .fold(0.0_f64, |current, glyph| current.max(glyph.width));

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
                if glyph.height > 0.0 {
                    (font_height / glyph.height) * glyph.width
                } else {
                    glyph.width
                }
            })
            .fold(0.0_f64, f64::max);
    } else if options.font_height.is_some() && max_glyph_height > 0.0 {
        font_width *= font_height / max_glyph_height;
    }
    let ascent = options.ascent.unwrap_or(font_height - descent);
    let font_id = options.font_id.unwrap_or(options.font_name).to_owned();
    let metadata = options.metadata.unwrap_or_default().to_owned();
    let optimize_output = options.optimize_output.unwrap_or(false);

    let mut processed_glyphs = glyphs
        .into_par_iter()
        .map(|glyph| {
            process_glyph(
                glyph,
                normalize,
                fixed_width,
                center_horizontally,
                center_vertically,
                ligature,
                round,
                max_glyph_height,
                font_height,
                font_width,
                descent,
                optimize_output,
            )
        })
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
    processed_glyphs.sort_by_key(|glyph| glyph.index);

    Ok(PreparedSvgFont {
        ascent,
        descent,
        font_height,
        font_id,
        font_width,
        metadata,
        processed_glyphs,
    })
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
    finalize_glyphs(options, glyphs)
}

pub(crate) fn source_content_hash(contents: &str) -> [u8; 16] {
    md5::compute(contents.as_bytes()).0
}

fn parse_glyphs_incremental(
    options: &SvgOptions,
    source_files: &[LoadedSvgFile],
    cache: &mut GlyphCache,
) -> Result<Vec<ParsedGlyph>, Error> {
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
            parse_svg_glyph(&work, preserve_aspect_ratio).map(|glyph| (index, glyph))
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
        let cached = CachedGlyph {
            height: glyph.height,
            paths: glyph.paths.clone(),
            width: glyph.width,
        };
        cache.by_content_hash.insert(hash, cached.clone());
        cache.content_hashes.insert(source_file.path.clone(), hash);
        cache.entries.insert(source_file.path.clone(), cached);
    }

    let active_hashes: HashSet<[u8; 16]> = cache.content_hashes.values().copied().collect();
    cache
        .by_content_hash
        .retain(|hash, _| active_hashes.contains(hash));

    // Assemble the full set: freshly-parsed glyphs by move, unchanged ones cloned from cache.
    let mut freshly_parsed: HashMap<usize, ParsedGlyph> = parsed.into_iter().collect();
    let mut glyphs = Vec::with_capacity(source_files.len());
    for (index, source_file) in source_files.iter().enumerate() {
        let glyph = match freshly_parsed.remove(&index) {
            Some(glyph) => glyph,
            None => {
                let cached = cache
                    .entries
                    .get(&source_file.path)
                    .expect("an unchanged file must have a cache entry");
                ParsedGlyph {
                    codepoint: codepoints[index],
                    height: cached.height,
                    index,
                    name: source_file.glyph_name.clone(),
                    paths: cached.paths.clone(),
                    width: cached.width,
                }
            }
        };
        glyphs.push(glyph);
    }
    glyphs.sort_by_key(|glyph| glyph.index);
    Ok(glyphs)
}
