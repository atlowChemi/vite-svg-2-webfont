use napi::{bindgen_prelude::Error, Status};
use usvg::tiny_skia_path::Rect;

use crate::svg::serialize::{append_path, optimize_path_data};
use crate::svg::types::{ParsedGlyph, ProcessedGlyph};

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_glyph(
    glyph: ParsedGlyph,
    normalize: bool,
    fixed_width: bool,
    center_horizontally: bool,
    center_vertically: bool,
    ligature: bool,
    round: f64,
    max_glyph_height: f64,
    font_height: f64,
    font_width: f64,
    descent: f64,
    optimize_output: bool,
) -> napi::Result<ProcessedGlyph> {
    let ratio = if normalize {
        let base = glyph.width.max(glyph.height);
        if base > 0.0 {
            font_height / base
        } else {
            1.0
        }
    } else if max_glyph_height > 0.0 {
        font_height / max_glyph_height
    } else {
        1.0
    };
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
        let transformed = path.transform(glyph_path_transform).ok_or_else(|| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to transform glyph '{}'.", glyph.name),
            )
        })?;
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
                        Error::new(
                            Status::GenericFailure,
                            format!("Failed to center glyph '{}'.", glyph.name),
                        )
                    })
                })
                .collect::<napi::Result<Vec<_>>>()?;
        }
    }
    let mut path_data = String::new();
    for path in &transformed_paths {
        append_path(&mut path_data, path, round);
    }
    // Trim trailing whitespace in-place (append_path always adds trailing spaces)
    let trimmed_len = path_data.trim_end().len();
    path_data.truncate(trimmed_len);
    let path_data = if optimize_output {
        optimize_path_data(&path_data)
    } else {
        path_data
    };

    let unicode_values = build_unicode_values(&glyph.name, glyph.codepoint, ligature);

    Ok(ProcessedGlyph {
        codepoint: glyph.codepoint,
        height: scaled_height,
        index: glyph.index,
        name: glyph.name,
        path_data,
        unicode_values,
        width: scaled_width,
    })
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

fn build_unicode_values(name: &str, codepoint: u32, ligature: bool) -> Vec<String> {
    let mut values = vec![format!("&#x{:X};", codepoint)];
    if ligature {
        let ligature_value = name
            .chars()
            .map(|character| format!("&#x{:X};", u32::from(character)))
            .collect::<String>();
        values.push(ligature_value);
    }
    values
}
