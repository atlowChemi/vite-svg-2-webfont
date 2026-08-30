#[cfg(test)]
mod tests;

use std::sync::Arc;

use rayon::join;

use crate::input::LoadedSvgFile;
use crate::svg::types::{GlyphCache, PreparedSvgFont, SvgOptions};
use crate::svg::{
    build_svg_font, prepare_svg_font, prepare_svg_font_incremental, svg_options_from_options,
};
use crate::ttf::{self, TtfGlyphCache};
use crate::types::{
    FontOutputs, FontType, GenerateWebfontsResult, RegenerationState,
    ResolvedGenerateWebfontsOptions,
};
use crate::{eot, sfnt, woff};

pub(crate) fn generate_webfonts_sync(
    options: ResolvedGenerateWebfontsOptions,
    source_files: Vec<LoadedSvgFile>,
) -> std::io::Result<GenerateWebfontsResult> {
    let svg_options = svg_options_from_options(&options);
    // When incremental, retain the parsed-glyph cache so a later `regenerate` can reuse the
    // glyphs whose source didn't change. Otherwise the geometry is dropped as soon as the font
    // is built, so one-shot builds carry no extra memory.
    let (prepared, glyph_cache, mut ttf_cache) = if options.incremental {
        let mut cache = GlyphCache::default();
        let prepared = prepare_svg_font_incremental(&svg_options, &source_files, &mut cache)?;
        (prepared, Some(cache), Some(TtfGlyphCache::default()))
    } else {
        (prepare_svg_font(&svg_options, &source_files)?, None, None)
    };
    let fonts = build_font_outputs(&options, &svg_options, &prepared, ttf_cache.as_mut())?;
    let regeneration_state = glyph_cache.map(|glyph_cache| RegenerationState {
        caches_dirty: false,
        glyph_cache,
        ttf_cache,
        written_outputs: std::collections::HashMap::new(),
    });

    Ok(GenerateWebfontsResult {
        cached: std::sync::OnceLock::new(),
        carried_render: None,
        css_context: None,
        fonts,
        html_context: None,
        options: std::sync::Arc::new(options),
        regeneration_state: std::sync::Arc::new(std::sync::Mutex::new(regeneration_state)),
        source_files: std::sync::Arc::new(source_files),
    })
}

/// Build every requested output format from an already-prepared glyph set.
pub(crate) fn build_font_outputs(
    options: &ResolvedGenerateWebfontsOptions,
    svg_options: &SvgOptions<'_>,
    prepared: &PreparedSvgFont,
    mut ttf_cache: Option<&mut TtfGlyphCache>,
) -> std::io::Result<FontOutputs> {
    let wants_svg = options.types.contains(&FontType::Svg);
    let wants_ttf = options.types.contains(&FontType::Ttf);
    let wants_woff = options.types.contains(&FontType::Woff);
    let wants_woff2 = options.types.contains(&FontType::Woff2);
    let wants_eot = options.types.contains(&FontType::Eot);

    let (svg_font, ttf_tables) = join(
        || -> std::io::Result<Option<String>> {
            if wants_svg {
                Ok(Some(build_svg_font(svg_options, prepared)))
            } else {
                Ok(None)
            }
        },
        || -> std::io::Result<Option<sfnt::SerializedFontTables>> {
            if wants_ttf || wants_woff || wants_woff2 || wants_eot {
                let ttf_options = ttf::ttf_options_from_options(options);
                match ttf_cache.as_deref_mut() {
                    Some(cache) => ttf::generate_ttf_font_from_glyphs_cached(
                        ttf_options,
                        &prepared.processed_glyphs,
                        cache,
                    )
                    .map(Some),
                    None => {
                        ttf::generate_ttf_font_from_glyphs(ttf_options, &prepared.processed_glyphs)
                            .map(Some)
                    }
                }
            } else {
                Ok(None)
            }
        },
    );

    let svg_font = svg_font?.map(Arc::new);
    let ttf_tables = ttf_tables?;

    let (ttf_font, woff_font, woff2_font, eot_font) = if let Some(ttf_tables) = ttf_tables {
        let woff_metadata = options
            .format_options
            .as_ref()
            .and_then(|value| value.woff.as_ref())
            .and_then(|value| value.metadata.as_deref());
        let woff2_quality = options
            .format_options
            .as_ref()
            .and_then(|value| value.woff2.as_ref())
            .and_then(|value| value.compression_quality)
            .unwrap_or(11);

        let ttf_tables = Arc::new(ttf_tables);
        let ttf_font = wants_ttf.then(|| ttf_tables.ttf_arc());
        let (woff1_cache, woff2_cache) = match ttf_cache {
            Some(cache) => {
                let (woff1, woff2) = cache.output_caches();
                (Some(woff1), Some(woff2))
            }
            None => (None, None),
        };
        let (woff_font, (woff2_font, eot_font)) = join(
            || -> std::io::Result<Option<Vec<u8>>> {
                if wants_woff {
                    match woff1_cache {
                        Some(cache) => {
                            woff::tables_to_woff1_cached(&ttf_tables, woff_metadata, cache)
                        }
                        None => woff::tables_to_woff1(&ttf_tables, woff_metadata),
                    }
                    .map(Some)
                } else {
                    Ok(None)
                }
            },
            || {
                join(
                    || -> std::io::Result<Option<Vec<u8>>> {
                        if wants_woff2 {
                            woff::tables_to_woff2(&ttf_tables, woff2_quality, woff2_cache).map(Some)
                        } else {
                            Ok(None)
                        }
                    },
                    || -> std::io::Result<Option<Vec<u8>>> {
                        if wants_eot {
                            eot::tables_to_eot(&ttf_tables).map(Some)
                        } else {
                            Ok(None)
                        }
                    },
                )
            },
        );

        (
            ttf_font,
            woff_font?.map(Arc::new),
            woff2_font?.map(Arc::new),
            eot_font?.map(Arc::new),
        )
    } else {
        (None, None, None, None)
    };

    Ok(FontOutputs {
        svg_font,
        ttf_font,
        woff_font,
        woff2_font,
        eot_font,
    })
}
