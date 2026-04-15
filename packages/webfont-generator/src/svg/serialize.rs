use std::fmt::Write as _;
use usvg::tiny_skia_path::{Path as TinyPath, PathSegment};

use crate::svg::types::{PreparedSvgFont, SvgOptions};

const DEFAULT_ROUNDING_PRECISION: f64 = 1_000_000_000_000.0;

pub(crate) fn build_svg_font(options: &SvgOptions, prepared: &PreparedSvgFont) -> String {
    let PreparedSvgFont {
        ascent,
        descent,
        font_height,
        font_id,
        font_width,
        metadata,
        processed_glyphs,
    } = prepared;
    let mut svg_font = String::from(
        r#"<?xml version="1.0" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd" >
<svg xmlns="http://www.w3.org/2000/svg">
"#,
    );
    if !metadata.is_empty() {
        _ = writeln!(svg_font, "<metadata>{metadata}</metadata>");
    }
    svg_font.push_str("<defs>\n");
    _ = writeln!(
        svg_font,
        "  <font id=\"{}\" horiz-adv-x=\"{font_width}\">",
        escape_xml(font_id)
    );
    _ = write!(
        svg_font,
        "    <font-face font-family=\"{}\"\n      units-per-em=\"{font_height}\" ascent=\"{ascent}\"\n      descent=\"{descent}\"",
        escape_xml(options.font_name),
    );
    if let Some(font_weight) = &options.font_weight {
        _ = write!(
            svg_font,
            "\n      font-weight=\"{}\"",
            escape_xml(font_weight)
        );
    }
    if let Some(font_style) = &options.font_style {
        _ = write!(
            svg_font,
            "\n      font-style=\"{}\"",
            escape_xml(font_style)
        );
    }
    svg_font.push_str(" />\n    <missing-glyph horiz-adv-x=\"0\" />\n");

    for glyph in processed_glyphs {
        for (index, unicode) in glyph.unicode_values.iter().enumerate() {
            if index == 0 {
                _ = write!(
                    svg_font,
                    "    <glyph glyph-name=\"{}\"\n      unicode=\"{unicode}\"\n      horiz-adv-x=\"{}\" d=\"{}\" />\n",
                    escape_xml(&glyph.name),
                    glyph.width,
                    escape_xml(&glyph.path_data),
                );
            } else {
                _ = write!(
                    svg_font,
                    "    <glyph glyph-name=\"{}-{index}\"\n      unicode=\"{unicode}\"\n      horiz-adv-x=\"{}\" d=\"{}\" />\n",
                    escape_xml(&glyph.name),
                    glyph.width,
                    escape_xml(&glyph.path_data),
                );
            }
        }
    }

    svg_font.push_str("  </font>\n</defs>\n</svg>\n");

    svg_font
}

pub(crate) fn append_path(target: &mut String, path: &TinyPath, round: f64) {
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(point) => {
                let _ = write!(
                    target,
                    "M {} {} ",
                    round_to_string(f64::from(point.x), round),
                    round_to_string(f64::from(point.y), round)
                );
            }
            PathSegment::LineTo(point) => {
                let _ = write!(
                    target,
                    "L {} {} ",
                    round_to_string(f64::from(point.x), round),
                    round_to_string(f64::from(point.y), round)
                );
            }
            PathSegment::QuadTo(control, point) => {
                let _ = write!(
                    target,
                    "Q {} {} {} {} ",
                    round_to_string(f64::from(control.x), round),
                    round_to_string(f64::from(control.y), round),
                    round_to_string(f64::from(point.x), round),
                    round_to_string(f64::from(point.y), round)
                );
            }
            PathSegment::CubicTo(control1, control2, point) => {
                let _ = write!(
                    target,
                    "C {} {} {} {} {} {} ",
                    round_to_string(f64::from(control1.x), round),
                    round_to_string(f64::from(control1.y), round),
                    round_to_string(f64::from(control2.x), round),
                    round_to_string(f64::from(control2.y), round),
                    round_to_string(f64::from(point.x), round),
                    round_to_string(f64::from(point.y), round)
                );
            }
            PathSegment::Close => target.push_str("Z "),
        }
    }
}

#[inline]
fn round_to_string(value: f64, round: f64) -> String {
    let precision = if round.is_finite() && round > 0.0 {
        round
    } else {
        DEFAULT_ROUNDING_PRECISION
    };
    let rounded = (value * precision).round() / precision;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        rounded.to_string()
    }
}

pub(crate) fn optimize_path_data(path_data: &str) -> String {
    use oxvg_path::{Path, convert::run as optimize_path, parser::Parse as _};

    let mut path = match Path::parse_string(path_data) {
        Ok(p) => p,
        Err(_) => return path_data.to_owned(),
    };
    optimize_path(
        &mut path,
        &oxvg_path::convert::Options::default(),
        &oxvg_path::convert::StyleInfo::default(),
    );
    path.to_string()
}

fn escape_xml(value: &str) -> String {
    // Fast path: most glyph/font names have no XML special chars
    if !value
        .bytes()
        .any(|b| matches!(b, b'&' | b'"' | b'<' | b'>'))
    {
        return value.to_owned();
    }
    let mut result = String::with_capacity(value.len() + 16);
    for ch in value.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            _ => result.push(ch),
        }
    }
    result
}
