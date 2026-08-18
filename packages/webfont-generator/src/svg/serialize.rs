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
        _ = write!(
            svg_font,
            "    <glyph glyph-name=\"{}\"\n      unicode=\"&#x{:X};\"\n      horiz-adv-x=\"{}\" d=\"{}\" />\n",
            escape_xml(&glyph.name),
            glyph.codepoint,
            glyph.width,
            escape_xml(&glyph.path_data),
        );
        if options.ligature {
            _ = write!(
                svg_font,
                "    <glyph glyph-name=\"{}-1\"\n      unicode=\"",
                escape_xml(&glyph.name),
            );
            for character in glyph.name.chars() {
                _ = write!(svg_font, "&#x{:X};", u32::from(character));
            }
            _ = writeln!(
                svg_font,
                "\"\n      horiz-adv-x=\"{}\" d=\"{}\" />",
                glyph.width,
                escape_xml(&glyph.path_data),
            );
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
                    RoundedCoordinate::new(point.x, round),
                    RoundedCoordinate::new(point.y, round)
                );
            }
            PathSegment::LineTo(point) => {
                let _ = write!(
                    target,
                    "L {} {} ",
                    RoundedCoordinate::new(point.x, round),
                    RoundedCoordinate::new(point.y, round)
                );
            }
            PathSegment::QuadTo(control, point) => {
                let _ = write!(
                    target,
                    "Q {} {} {} {} ",
                    RoundedCoordinate::new(control.x, round),
                    RoundedCoordinate::new(control.y, round),
                    RoundedCoordinate::new(point.x, round),
                    RoundedCoordinate::new(point.y, round)
                );
            }
            PathSegment::CubicTo(control1, control2, point) => {
                let _ = write!(
                    target,
                    "C {} {} {} {} {} {} ",
                    RoundedCoordinate::new(control1.x, round),
                    RoundedCoordinate::new(control1.y, round),
                    RoundedCoordinate::new(control2.x, round),
                    RoundedCoordinate::new(control2.y, round),
                    RoundedCoordinate::new(point.x, round),
                    RoundedCoordinate::new(point.y, round)
                );
            }
            PathSegment::Close => target.push_str("Z "),
        }
    }
}

struct RoundedCoordinate(f64);

impl RoundedCoordinate {
    #[inline]
    fn new(value: f32, round: f64) -> Self {
        let precision = if round.is_finite() && round > 0.0 {
            round
        } else {
            DEFAULT_ROUNDING_PRECISION
        };
        Self((f64::from(value) * precision).round() / precision)
    }
}

impl std::fmt::Display for RoundedCoordinate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.fract() == 0.0 {
            write!(formatter, "{:.0}", self.0)
        } else {
            self.0.fmt(formatter)
        }
    }
}

pub(crate) fn optimize_path_data(path_data: &str) -> String {
    use oxvg_path::{Path, geometry::Tolerance, optimize::Options, parser::Parse as _};

    let mut path = match Path::parse_string(path_data) {
        Ok(p) => p,
        Err(_) => return path_data.to_owned(),
    };
    let options = Options::all()
        .difference(Options::CloseSegments | Options::RemoveCloseLine | Options::UniteSegments);
    path = path.optimize(options, &Tolerance::default());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_coordinate_matches_allocating_formatter() {
        let values = [
            -0.0,
            0.0,
            -1.5,
            0.004_999_999_9,
            0.005_000_000_4,
            1.234_567_8,
            16_777_216.0,
            f32::MAX,
            f32::MIN_POSITIVE,
        ];
        let precisions = [
            1.0,
            10.0,
            100.0,
            1_000.0,
            DEFAULT_ROUNDING_PRECISION,
            10e12,
            1e40,
            0.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
        ];

        for value in values {
            for precision in precisions {
                let expected = round_to_string(value, precision);
                assert_eq!(
                    RoundedCoordinate::new(value, precision).to_string(),
                    expected
                );
            }
        }
    }

    fn round_to_string(value: f32, round: f64) -> String {
        let precision = if round.is_finite() && round > 0.0 {
            round
        } else {
            DEFAULT_ROUNDING_PRECISION
        };
        let rounded = (f64::from(value) * precision).round() / precision;
        if rounded.fract() == 0.0 {
            format!("{rounded:.0}")
        } else {
            rounded.to_string()
        }
    }
}
