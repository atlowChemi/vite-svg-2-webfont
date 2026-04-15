mod parse;
mod process;
mod serialize;
#[cfg(test)]
mod tests;
pub(crate) mod types;

use rayon::prelude::*;
use std::io::{Error, ErrorKind};

use parse::parse_svg_glyph;
use process::process_glyph;
pub(crate) use serialize::build_svg_font;
use types::{GlyphWorkItem, PreparedSvgFont, SvgOptions};

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
    if source_files.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Expected at least one SVG file for native generation.",
        ));
    }

    let mut work_items = Vec::with_capacity(source_files.len());
    let normalize = options.normalize;
    let fixed_width = options.fixed_width.unwrap_or(false);
    let center_horizontally = options.center_horizontally.unwrap_or(false);
    let center_vertically = options.center_vertically.unwrap_or(false);
    let ligature = options.ligature;
    let preserve_aspect_ratio = options.preserve_aspect_ratio.unwrap_or(false);
    let round = options.round.unwrap_or(10e12);

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
