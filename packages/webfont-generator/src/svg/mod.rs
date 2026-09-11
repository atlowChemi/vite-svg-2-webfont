mod geometry;
mod incremental;
mod parse;
mod process;
mod serialize;
#[cfg(test)]
mod tests;
pub(crate) mod types;
mod winding;

use rayon::prelude::*;
use std::io::{Error, ErrorKind};

pub(crate) use incremental::{prepare_svg_font_incremental, source_content_hash};
use parse::parse_svg_glyph;
use process::{glyph_scale, process_glyph};
pub(crate) use serialize::build_svg_font;
pub(crate) use serialize::rounded_coordinate;
use types::{
    GlyphWorkItem, ParsedGlyph, PreparedSvgFont, PreparedVariantFamily, ProcessedGlyph,
    ProcessedVariantGlyph, SvgOptions,
};

use crate::input::{
    LoadedSvgFile, ResolvedGenerateWebfontsOptions, VariantFamilySources, VariantGlyphSource,
};
use crate::types::FontType;

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

pub(crate) fn prepare_variant_svg_family(
    options: &SvgOptions,
    family: &VariantFamilySources,
) -> Result<PreparedVariantFamily, Error> {
    let parsed_variants = family
        .variants
        .iter()
        .map(|files| parse_glyphs(options, files))
        .collect::<Result<Vec<_>, _>>()?;
    let parsed = parsed_variants.iter().flatten().collect::<Vec<_>>();
    let plan = finalize_plan(options, &parsed, |glyph| glyph.height, |glyph| glyph.width);
    let mut advances = family
        .glyphs
        .iter()
        .map(|glyph| {
            glyph
                .sources
                .iter()
                .filter_map(
                    |source| match source.expect("missing glyphs must be resolved") {
                        VariantGlyphSource::Source {
                            variant_index,
                            source_index,
                        } => {
                            let parsed = &parsed_variants[variant_index][source_index];
                            Some(
                                parsed.width
                                    * glyph_scale(
                                        parsed.width,
                                        parsed.height,
                                        plan.normalize,
                                        plan.max_glyph_height,
                                        plan.font_height,
                                    ),
                            )
                        }
                        VariantGlyphSource::Blank => None,
                    },
                )
                .fold(0.0_f64, f64::max)
        })
        .collect::<Vec<_>>();
    if plan.fixed_width {
        let family_advance = advances.iter().copied().fold(0.0_f64, f64::max);
        advances.fill(family_advance);
    }

    let glyphs = family
        .glyphs
        .iter()
        .zip(advances)
        .enumerate()
        .map(|(glyph_index, (glyph, advance_width))| {
            let outlines = glyph
                .sources
                .iter()
                .map(
                    |source| match source.expect("missing glyphs must be resolved") {
                        VariantGlyphSource::Source {
                            variant_index,
                            source_index,
                        } => {
                            let mut parsed = parsed_variants[variant_index][source_index].clone();
                            parsed.name.clone_from(&glyph.name);
                            parsed.codepoint = glyph.codepoint;
                            parsed.index = glyph_index;
                            process_glyph_with_advance(parsed, &plan, advance_width).map(Some)
                        }
                        VariantGlyphSource::Blank => Ok(None),
                    },
                )
                .collect::<Result<Box<[_]>, Error>>()?;
            Ok(ProcessedVariantGlyph {
                name: glyph.name.clone(),
                codepoint: glyph.codepoint,
                advance_width,
                outlines,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(PreparedVariantFamily {
        ascent: plan.ascent,
        descent: plan.descent,
        font_height: plan.font_height,
        glyphs,
    })
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

fn process_glyph_with_advance(
    glyph: ParsedGlyph,
    plan: &FinalizePlan,
    advance_width: f64,
) -> Result<ProcessedGlyph, Error> {
    process_glyph(
        glyph,
        plan.normalize,
        true,
        plan.center_horizontally,
        plan.center_vertically,
        plan.round,
        plan.max_glyph_height,
        plan.font_height,
        advance_width,
        plan.descent,
        plan.optimize_output,
        plan.serialize_path,
        plan.structure_path,
    )
}
