use std::io::Error;
use std::sync::Arc;
use usvg::tiny_skia_path::Rect;

use super::geometry::{bezpath_from_oxvg_path, bezpath_hash, rounded_bezpath_from_tiny_paths};
use crate::svg::serialize::{append_path, optimize_path};
use crate::svg::types::{ParsedGlyph, ProcessedGlyph};
use crate::svg::winding::normalize_winding;

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_glyph(
    glyph: ParsedGlyph,
    normalize: bool,
    fixed_width: bool,
    center_horizontally: bool,
    center_vertically: bool,
    round: f64,
    max_glyph_height: f64,
    font_height: f64,
    font_width: f64,
    descent: f64,
    optimize_output: bool,
    serialize_path: bool,
    structure_path: bool,
) -> Result<ProcessedGlyph, Error> {
    let ratio = glyph_scale(
        glyph.width,
        glyph.height,
        normalize,
        max_glyph_height,
        font_height,
    );
    let mut scaled_width = glyph.width * ratio;
    let scaled_height = glyph.height * ratio;
    let y_offset = scaled_height - descent;
    let glyph_path_transform = usvg::Transform::from_row(
        ratio as f32,
        0.0,
        0.0,
        -(ratio as f32),
        0.0,
        y_offset as f32,
    );

    let mut transformed_paths = Vec::with_capacity(glyph.paths.len());
    for path in glyph.paths {
        let transformed = path
            .transform(glyph_path_transform)
            .ok_or_else(|| Error::other(format!("Failed to transform glyph '{}'.", glyph.name)))?;
        transformed_paths.push(transformed);
    }
    if fixed_width {
        scaled_width = font_width;
    }
    if center_horizontally || center_vertically {
        let bounds = calculate_combined_bounds(&transformed_paths);
        let translate_x = if center_horizontally {
            (scaled_width - f64::from(bounds.width())) / 2.0 - f64::from(bounds.left())
        } else {
            0.0
        };
        let translate_y = if center_vertically {
            (font_height - f64::from(bounds.height())) / 2.0 - f64::from(bounds.top()) - descent
        } else {
            0.0
        };
        if translate_x != 0.0 || translate_y != 0.0 {
            let translate = usvg::Transform::from_translate(translate_x as f32, translate_y as f32);
            transformed_paths = transformed_paths
                .into_iter()
                .map(|path| {
                    path.transform(translate).ok_or_else(|| {
                        Error::other(format!("Failed to center glyph '{}'.", glyph.name))
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
        }
    }
    // Apply the monochrome icon-font containment heuristic: nested contours alternate winding so
    // foreground-on-background SVG layers become knockouts. No-op glyphs pass through byte-identical.
    let transformed_paths = normalize_winding(transformed_paths);
    let mut path_data = if serialize_path || optimize_output {
        let mut path_data = String::new();
        for path in &transformed_paths {
            append_path(&mut path_data, path, round);
        }
        path_data.truncate(path_data.trim_end().len());
        path_data
    } else {
        String::new()
    };
    let ttf_path = if optimize_output {
        match optimize_path(&path_data) {
            Some(optimized) => {
                let ttf_path = structure_path.then(|| Arc::new(bezpath_from_oxvg_path(&optimized)));
                if serialize_path {
                    path_data = optimized.to_string();
                } else {
                    path_data.clear();
                }
                ttf_path
            }
            None => None,
        }
    } else {
        structure_path.then(|| Arc::new(rounded_bezpath_from_tiny_paths(&transformed_paths, round)))
    };
    let ttf_path_hash = ttf_path.as_deref().map(bezpath_hash);

    Ok(ProcessedGlyph {
        codepoint: glyph.codepoint,
        height: scaled_height,
        index: glyph.index,
        name: glyph.name,
        path_data: path_data.into(),
        ttf_path,
        ttf_path_hash,
        width: scaled_width,
    })
}

pub(crate) fn glyph_scale(
    width: f64,
    height: f64,
    normalize: bool,
    max_glyph_height: f64,
    font_height: f64,
) -> f64 {
    if normalize {
        let base = width.max(height);
        if base <= 0.0 {
            return 1.0;
        }
        font_height / base
    } else if max_glyph_height > 0.0 {
        font_height / max_glyph_height
    } else {
        1.0
    }
}

fn calculate_combined_bounds(paths: &[usvg::tiny_skia_path::Path]) -> Rect {
    let mut left = f32::INFINITY;
    let mut top = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::NEG_INFINITY;

    for path in paths {
        let bounds = path.compute_tight_bounds().unwrap_or_else(|| path.bounds());
        left = left.min(bounds.left());
        top = top.min(bounds.top());
        right = right.max(bounds.right());
        bottom = bottom.max(bounds.bottom());
    }

    Rect::from_ltrb(left, top, right, bottom)
        .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, 1.0, 1.0).expect("fallback rect"))
}
