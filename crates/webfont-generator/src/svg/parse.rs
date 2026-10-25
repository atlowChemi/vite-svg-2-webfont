use std::borrow::Cow;
use std::io::{Error, ErrorKind};
use usvg::tiny_skia_path::Path as TinyPath;

use crate::svg::types::{GlyphWorkItem, ParsedColorLayer, ParsedGlyph};
use crate::types::color::ResolvedLayerPaint;

mod color;

pub(crate) fn parse_svg_glyph(
    item: &GlyphWorkItem,
    preserve_aspect_ratio: bool,
    options: &usvg::Options,
    color: bool,
) -> Result<ParsedGlyph, Error> {
    let svg = item.source_file.contents.as_ref();
    let document = parse_svg_document(svg)?;
    let svg = root_aspect_ratio(svg, &document, preserve_aspect_ratio)?;
    let document = match &svg {
        Cow::Owned(rewritten) => parse_svg_document(rewritten)?,
        Cow::Borrowed(_) => document,
    };
    let svg = svg.as_ref();
    let tree = usvg::Tree::from_xmltree(&document, options).map_err(|error| {
        Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Failed to parse SVG fixture '{}': {error}",
                item.source_file.id
            ),
        )
    })?;
    let (tree, marker) = if color {
        let (tree, marker) =
            color::marked_tree(svg, &document, tree, options).map_err(|error| {
                Error::new(
                    error.kind(),
                    format!(
                        "Failed to extract SVG paint for '{}': {error}",
                        item.source_file.id
                    ),
                )
            })?;
        (tree, marker)
    } else {
        (tree, None)
    };
    let mut paths = Vec::new();
    let mut color_layers = color.then(Vec::new);
    collect_paths(tree.root(), &mut paths, marker, &mut color_layers)?;

    Ok(ParsedGlyph {
        color_layers: color_layers.map(Into::into),
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
    paths: &mut Vec<TinyPath>,
    marker: Option<usvg::Color>,
    color_layers: &mut Option<Vec<ParsedColorLayer>>,
) -> Result<(), Error> {
    for node in group.children() {
        match node {
            usvg::Node::Group(child_group) => {
                collect_paths(child_group, paths, marker, color_layers)?
            }
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
                    Error::other("Failed to apply an absolute transform to a glyph path.")
                })?;
                if let Some(layers) = color_layers
                    && path.is_visible()
                    && let Some(fill) = path.fill()
                    && let usvg::Paint::Color(color) = fill.paint()
                {
                    let alpha = fill.opacity().get();
                    let paint = if Some(*color) == marker {
                        ResolvedLayerPaint::Foreground { alpha }
                    } else {
                        ResolvedLayerPaint::Solid {
                            red: color.red,
                            green: color.green,
                            blue: color.blue,
                            alpha,
                        }
                    };
                    layers.push(ParsedColorLayer {
                        path_index: paths.len(),
                        paint,
                        fill_rule: fill.rule(),
                    });
                }
                paths.push(transformed);
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_svg_document(svg: &str) -> Result<roxmltree::Document<'_>, Error> {
    roxmltree::Document::parse_with_options(
        svg,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .map_err(|error| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Failed to inspect SVG root element: {error}"),
        )
    })
}

fn root_aspect_ratio<'a>(
    svg: &'a str,
    document: &roxmltree::Document<'_>,
    preserve: bool,
) -> Result<Cow<'a, str>, Error> {
    let root = document.root_element();
    if !root.has_tag_name("svg") {
        return Ok(Cow::Borrowed(svg));
    }

    let Some(view_box) = root.attribute("viewBox") else {
        return Ok(Cow::Borrowed(svg));
    };
    parse_view_box(view_box).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "Failed to parse the SVG viewBox for native generation.",
        )
    })?;
    let desired = if preserve { "xMidYMid meet" } else { "none" };
    let attribute = root.attribute_node("preserveAspectRatio");
    let current_preserve = !attribute
        .map(|attr| attr.value())
        .unwrap_or("xMidYMid meet")
        .trim_start()
        .starts_with("none");
    if preserve == current_preserve {
        return Ok(Cow::Borrowed(svg));
    }

    // Let usvg compose the requested viewBox mapping with root transforms and
    // transform-origin. Correcting already-transformed paths changes their order.
    let mut rewritten = svg.to_owned();
    if let Some(attribute) = attribute {
        rewritten.replace_range(attribute.range_value(), desired);
    } else {
        let start = root.range().start + 1;
        let end = start
            + svg[start..]
                .find(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>'))
                .ok_or_else(|| Error::other("Missing SVG root name terminator."))?;
        rewritten.insert_str(end, &format!(r#" preserveAspectRatio="{desired}""#));
    }
    Ok(Cow::Owned(rewritten))
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

#[cfg(test)]
mod tests;
