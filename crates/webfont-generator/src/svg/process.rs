use std::io::Error;
use std::sync::Arc;
use usvg::tiny_skia_path::Rect;

use super::geometry::{bezpath_from_oxvg_path, bezpath_hash, rounded_bezpath_from_tiny_paths};
use crate::svg::serialize::{append_path, commands_from_paths, optimize_commands};
use crate::svg::types::{ParsedGlyph, ProcessedGlyph};
use crate::svg::winding::{normalize_evenodd, normalize_winding};
use crate::types::color::ProcessedColorLayer;

#[expect(clippy::too_many_arguments)]
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
    let color_layers = glyph.color_layers.map(|layers| {
        layers
            .iter()
            .map(|layer| {
                let path = &transformed_paths[layer.path_index];
                let paths = if layer.fill_rule == usvg::FillRule::EvenOdd {
                    normalize_evenodd(path)
                } else {
                    std::borrow::Cow::Borrowed(std::slice::from_ref(path))
                };
                let outline = if optimize_output {
                    commands_from_paths(&paths, round)
                        .map(optimize_commands)
                        .map(|path| bezpath_from_oxvg_path(&path))
                        .unwrap_or_else(|| rounded_bezpath_from_tiny_paths(&paths, round))
                } else {
                    rounded_bezpath_from_tiny_paths(&paths, round)
                };
                ProcessedColorLayer {
                    outline_hash: bezpath_hash(&outline),
                    outline: Arc::new(outline),
                    paint: layer.paint,
                }
            })
            .collect::<Arc<[_]>>()
    });
    // Apply the monochrome icon-font containment heuristic: nested contours alternate winding so
    // foreground-on-background SVG layers become knockouts. No-op glyphs pass through byte-identical.
    let transformed_paths = normalize_winding(transformed_paths);
    let direct = optimize_output
        .then(|| commands_from_paths(&transformed_paths, round))
        .flatten()
        .map(optimize_commands);
    let mut path_data = if direct.is_none() && (serialize_path || optimize_output) {
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
        match direct {
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
        color_layers,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::serialize::route_counts;
    use crate::svg::types::ParsedColorLayer;
    use crate::types::color::ResolvedLayerPaint;
    use oxvg_path::{Path, parser::Parse as _};
    use usvg::tiny_skia_path::PathBuilder;

    #[test]
    fn optimized_processing_uses_direct_commands_and_preserves_overflow_rejection() {
        for color in [false, true] {
            for serialize in [false, true] {
                for structure in [false, true] {
                    for round in [1e12, f64::MAX] {
                        let mut builder = PathBuilder::new();
                        builder.move_to(2.0, 2.0);
                        builder.line_to(8.0, 2.0);
                        builder.line_to(8.0, 8.0);
                        builder.close();
                        let glyph = ParsedGlyph {
                            color_layers: color.then(|| {
                                Arc::from([ParsedColorLayer {
                                    path_index: 0,
                                    paint: ResolvedLayerPaint::Foreground { alpha: 1.0 },
                                    fill_rule: usvg::FillRule::NonZero,
                                }])
                            }),
                            codepoint: 0xe001,
                            height: 10.0,
                            index: 0,
                            name: "direct-route".into(),
                            paths: vec![builder.finish().unwrap()],
                            width: 10.0,
                        };
                        let process = |glyph, optimize, serialize, structure| {
                            process_glyph(
                                glyph, false, false, false, false, round, 10.0, 10.0, 10.0, 0.0,
                                optimize, serialize, structure,
                            )
                            .unwrap()
                        };
                        // Reproduce the old serialized input with the same transforms/rounding.
                        let legacy = process(glyph.clone(), false, true, true);
                        let parsed = Path::parse_string(&legacy.path_data).ok();
                        route_counts::DIRECT.with(|count| count.set(0));
                        route_counts::SERIALIZED.with(|count| count.set(0));
                        let result = process(glyph, true, serialize, structure);
                        assert_eq!(
                            route_counts::DIRECT.with(|count| count.get()),
                            1 + usize::from(color)
                        );
                        if round == f64::MAX {
                            assert!(
                                parsed.is_none(),
                                "the parser cannot rescue rounding overflow"
                            );
                            assert!(result.ttf_path.is_none());
                            assert!(result.ttf_path_hash.is_none());
                            assert_eq!(result.path_data, legacy.path_data);
                            if color {
                                let outline = &result.color_layers.as_ref().unwrap()[0].outline;
                                assert!(!outline.is_finite());
                                assert_eq!(
                                    result.color_layers.as_ref().unwrap()[0].outline_hash,
                                    legacy.color_layers.as_ref().unwrap()[0].outline_hash,
                                );
                            }
                        } else {
                            assert_eq!(route_counts::SERIALIZED.with(|count| count.get()), 0);
                            let optimized = optimize_commands(parsed.unwrap());
                            assert_eq!(
                                result.path_data.as_ref(),
                                if serialize {
                                    optimized.to_string()
                                } else {
                                    String::new()
                                }
                            );
                            assert_eq!(result.ttf_path.is_some(), structure);
                            if let Some(path) = result.ttf_path {
                                assert_eq!(*path, bezpath_from_oxvg_path(&optimized));
                            }
                            if color {
                                assert!(
                                    result.color_layers.as_ref().unwrap()[0].outline.is_finite()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
