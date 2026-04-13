use napi::{bindgen_prelude::Error, Status};
use usvg::tiny_skia_path::Path as TinyPath;
use usvg::Transform;

use crate::svg::types::{GlyphWorkItem, ParsedGlyph};

struct RootSvgMetrics {
    current_preserve_aspect_ratio: bool,
    view_box_height: f64,
    view_box_width: f64,
    view_box_x: f64,
    view_box_y: f64,
    viewport_height: f64,
    viewport_width: f64,
}

pub(crate) fn parse_svg_glyph(
    item: &GlyphWorkItem,
    preserve_aspect_ratio: bool,
) -> napi::Result<ParsedGlyph> {
    let svg = item.source_file.contents.as_bytes();
    let root_metrics = parse_root_svg_metrics(svg)?;
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg, &options).map_err(|error| {
        Error::new(
            Status::InvalidArg,
            format!(
                "Failed to parse SVG fixture '{}': {error}",
                item.source_file.path
            ),
        )
    })?;
    let mut paths = Vec::new();
    let root_correction = if let Some(metrics) = root_metrics.as_ref() {
        build_root_viewbox_correction(metrics, preserve_aspect_ratio)?
    } else {
        None
    };

    collect_paths(tree.root(), root_correction, &mut paths)?;

    Ok(ParsedGlyph {
        codepoint: item.codepoint,
        height: tree.size().height() as f64,
        index: item.index,
        name: item.name.to_owned(),
        paths,
        width: tree.size().width() as f64,
    })
}

fn collect_paths(
    group: &usvg::Group,
    root_correction: Option<Transform>,
    paths: &mut Vec<TinyPath>,
) -> napi::Result<()> {
    for node in group.children() {
        match node {
            usvg::Node::Group(child_group) => collect_paths(child_group, root_correction, paths)?,
            usvg::Node::Path(path) => {
                // Convert stroke-only paths to filled outlines (matching upstream svgicons2svgfont behavior)
                let path_data = if path.fill().is_some() {
                    path.data().clone()
                } else if let Some(stroke) = path.stroke() {
                    let tiny_stroke = usvg::tiny_skia_path::Stroke {
                        width: stroke.width().get(),
                        miter_limit: stroke.miterlimit().get(),
                        line_cap: match stroke.linecap() {
                            usvg::LineCap::Butt => usvg::tiny_skia_path::LineCap::Butt,
                            usvg::LineCap::Round => usvg::tiny_skia_path::LineCap::Round,
                            usvg::LineCap::Square => usvg::tiny_skia_path::LineCap::Square,
                        },
                        line_join: match stroke.linejoin() {
                            usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => {
                                usvg::tiny_skia_path::LineJoin::Miter
                            }
                            usvg::LineJoin::Round => usvg::tiny_skia_path::LineJoin::Round,
                            usvg::LineJoin::Bevel => usvg::tiny_skia_path::LineJoin::Bevel,
                        },
                        dash: stroke.dasharray().and_then(|array| {
                            usvg::tiny_skia_path::StrokeDash::new(
                                array.to_vec(),
                                stroke.dashoffset(),
                            )
                        }),
                    };
                    match path.data().stroke(&tiny_stroke, 1.0) {
                        Some(outlined) => outlined,
                        None => continue,
                    }
                } else {
                    continue;
                };

                let transformed = path_data.transform(path.abs_transform()).ok_or_else(|| {
                    Error::new(
                        Status::GenericFailure,
                        "Failed to apply an absolute transform to a glyph path.",
                    )
                })?;
                let transformed = if let Some(root_correction) = root_correction {
                    transformed.transform(root_correction).ok_or_else(|| {
                        Error::new(
                            Status::GenericFailure,
                            "Failed to apply a root viewBox correction to a glyph path.",
                        )
                    })?
                } else {
                    transformed
                };
                paths.push(transformed);
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_root_svg_metrics(svg: &[u8]) -> napi::Result<Option<RootSvgMetrics>> {
    let svg_text = std::str::from_utf8(svg).map_err(|error| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to decode SVG fixture as UTF-8: {error}"),
        )
    })?;
    let document = roxmltree::Document::parse_with_options(
        svg_text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .map_err(|error| {
        Error::new(
            Status::InvalidArg,
            format!("Failed to inspect SVG root element: {error}"),
        )
    })?;
    let root = document.root_element();
    if !root.has_tag_name("svg") {
        return Ok(None);
    }

    let Some(view_box) = root.attribute("viewBox") else {
        return Ok(None);
    };
    let values = parse_view_box(view_box).ok_or_else(|| {
        Error::new(
            Status::InvalidArg,
            "Failed to parse the SVG viewBox for native generation.",
        )
    })?;
    let viewport_width = root
        .attribute("width")
        .and_then(parse_number_prefix)
        .unwrap_or(values.2);
    let viewport_height = root
        .attribute("height")
        .and_then(parse_number_prefix)
        .unwrap_or(values.3);
    let current_preserve_aspect_ratio = root
        .attribute("preserveAspectRatio")
        .map(|value| !value.trim_start().starts_with("none"))
        .unwrap_or(true);

    Ok(Some(RootSvgMetrics {
        current_preserve_aspect_ratio,
        view_box_height: values.3,
        view_box_width: values.2,
        view_box_x: values.0,
        view_box_y: values.1,
        viewport_height,
        viewport_width,
    }))
}

fn build_root_viewbox_correction(
    metrics: &RootSvgMetrics,
    preserve_aspect_ratio: bool,
) -> napi::Result<Option<Transform>> {
    let current = root_viewbox_transform(metrics, metrics.current_preserve_aspect_ratio);
    let desired = root_viewbox_transform(metrics, preserve_aspect_ratio);

    if transforms_close(current, desired) {
        return Ok(None);
    }

    let inverse_current = current.invert().ok_or_else(|| {
        Error::new(
            Status::GenericFailure,
            "Failed to invert the current root viewBox transform.",
        )
    })?;

    Ok(Some(concat_transforms(desired, inverse_current)))
}

fn root_viewbox_transform(metrics: &RootSvgMetrics, preserve_aspect_ratio: bool) -> Transform {
    let sx = metrics.viewport_width / metrics.view_box_width;
    let sy = metrics.viewport_height / metrics.view_box_height;
    let (sx, sy) = if preserve_aspect_ratio {
        let scale = sx.min(sy);
        (scale, scale)
    } else {
        (sx, sy)
    };

    let x = -metrics.view_box_x * sx;
    let y = -metrics.view_box_y * sy;
    let w = metrics.viewport_width - metrics.view_box_width * sx;
    let h = metrics.viewport_height - metrics.view_box_height * sy;
    let tx = if preserve_aspect_ratio {
        x + w / 2.0
    } else {
        x
    };
    let ty = if preserve_aspect_ratio {
        y + h / 2.0
    } else {
        y
    };

    Transform::from_row(sx as f32, 0.0, 0.0, sy as f32, tx as f32, ty as f32)
}

fn concat_transforms(a: Transform, b: Transform) -> Transform {
    Transform {
        sx: a.sx * b.sx + a.kx * b.ky,
        kx: a.sx * b.kx + a.kx * b.sy,
        tx: a.sx * b.tx + a.kx * b.ty + a.tx,
        ky: a.ky * b.sx + a.sy * b.ky,
        sy: a.ky * b.kx + a.sy * b.sy,
        ty: a.ky * b.tx + a.sy * b.ty + a.ty,
    }
}

fn transforms_close(a: Transform, b: Transform) -> bool {
    const EPSILON: f32 = 0.0001;
    (a.sx - b.sx).abs() < EPSILON
        && (a.kx - b.kx).abs() < EPSILON
        && (a.ky - b.ky).abs() < EPSILON
        && (a.sy - b.sy).abs() < EPSILON
        && (a.tx - b.tx).abs() < EPSILON
        && (a.ty - b.ty).abs() < EPSILON
}

fn parse_view_box(value: &str) -> Option<(f64, f64, f64, f64)> {
    let values = value
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    if values.len() != 4 {
        return None;
    }

    Some((values[0], values[1], values[2], values[3]))
}

fn parse_number_prefix(value: &str) -> Option<f64> {
    let trimmed = value.trim_start();
    let mut end = 0;
    let mut seen_digit = false;

    for (index, character) in trimmed.char_indices() {
        let allowed = character.is_ascii_digit()
            || character == '+'
            || character == '-'
            || character == '.'
            || character == 'e'
            || character == 'E';
        if !allowed {
            break;
        }
        if character.is_ascii_digit() {
            seen_digit = true;
        }
        end = index + character.len_utf8();
    }

    if !seen_digit || end == 0 {
        return None;
    }

    trimmed[..end].parse::<f64>().ok()
}
