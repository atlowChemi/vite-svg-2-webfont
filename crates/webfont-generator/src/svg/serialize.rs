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
    let header = r#"<?xml version="1.0" standalone="no"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd" >
<svg xmlns="http://www.w3.org/2000/svg">
"#;
    let estimated_capacity = header.len()
        + metadata.len()
        + 256
        + processed_glyphs
            .iter()
            .map(|glyph| {
                glyph.name.len()
                    + glyph.path_data.len()
                    + 96
                    + usize::from(options.ligature)
                        * (glyph.name.len() * 11 + glyph.path_data.len() + 96)
            })
            .sum::<usize>();
    let mut svg_font = String::with_capacity(estimated_capacity);
    svg_font.push_str(header);
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
    #[cfg(test)]
    route_counts::SERIALIZED.with(|count| count.set(count.get() + 1));

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
        Self(rounded_coordinate(value, round))
    }
}

pub(crate) fn rounded_coordinate(value: f32, round: f64) -> f64 {
    let precision = if round.is_finite() && round > 0.0 {
        round
    } else {
        DEFAULT_ROUNDING_PRECISION
    };
    (f64::from(value) * precision).round() / precision
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

/// Construct the same rounded command stream as `append_path` followed by oxvg's parser.
/// Reject non-finite rounding results, whose serialized numbers the parser also rejects.
pub(crate) fn commands_from_paths(paths: &[TinyPath], round: f64) -> Option<oxvg_path::Path> {
    use oxvg_path::command::{Data, ID};

    #[cfg(test)]
    route_counts::DIRECT.with(|count| count.set(count.get() + 1));
    let mut result = Vec::new();
    let mut next = ID::None;
    for segment in paths.iter().flat_map(|path| path.segments()) {
        let coordinate = |value| rounded_coordinate(value, round);
        let data = match segment {
            PathSegment::MoveTo(p) => Data::MoveTo([p.x, p.y].map(coordinate)),
            PathSegment::LineTo(p) => Data::LineTo([p.x, p.y].map(coordinate)),
            PathSegment::QuadTo(c, p) => {
                Data::QuadraticBezierTo([c.x, c.y, p.x, p.y].map(coordinate))
            }
            PathSegment::CubicTo(a, b, p) => {
                Data::CubicBezierTo([a.x, a.y, b.x, b.y, p.x, p.y].map(coordinate))
            }
            PathSegment::Close => Data::ClosePath,
        };
        if data
            .args()
            .is_some_and(|args| args.iter().any(|v| !v.is_finite()))
        {
            return None;
        }
        let id = data.id();
        let implicit = next == id;
        next = id.next_implicit();
        // The parser wraps even explicitly written repeated commands (and L after M).
        result.push(if implicit {
            Data::Implicit(Box::new(data))
        } else {
            data
        });
    }
    Some(oxvg_path::Path(result))
}

pub(crate) fn optimize_commands(path: oxvg_path::Path) -> oxvg_path::Path {
    use oxvg_path::{geometry::Tolerance, optimize::Options};
    let options = Options::all()
        .difference(Options::CloseSegments | Options::RemoveCloseLine | Options::UniteSegments);
    path.optimize(options, &Tolerance::default())
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
pub(super) mod route_counts {
    use std::cell::Cell;

    // Processing tests run synchronously; thread-local counts isolate parallel tests.
    thread_local! {
        pub static DIRECT: Cell<usize> = const { Cell::new(0) };
        pub static SERIALIZED: Cell<usize> = const { Cell::new(0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_input_matches_parser_bits_and_optimized_output() {
        use oxvg_path::{Path, parser::Parse as _};
        use usvg::tiny_skia_path::PathBuilder;

        let mut builder = PathBuilder::new();
        builder.move_to(-0.0, 0.0);
        builder.line_to(-0.004_999_999_9, 0.005_000_000_4);
        builder.line_to(16_777_216.0, 100_000_000.0);
        builder.quad_to(1.234_567_8, -1.5, 2.0, 3.0);
        builder.quad_to(4.0, 5.0, 6.0, 7.0);
        builder.cubic_to(8.0, 9.0, 10.0, 11.0, 12.0, 13.0);
        builder.cubic_to(14.0, 15.0, 16.0, 17.0, 18.0, 19.0);
        builder.close();
        builder.move_to(2.0, f32::MIN_POSITIVE);
        builder.line_to(3.0, 4.0);
        builder.close();
        let path = builder.finish().unwrap();
        let mut large = PathBuilder::new();
        // Keep TinyPath's bounds representable while exercising rounding overflow.
        large.move_to(f32::MAX / 4.0, -f32::MAX / 4.0);
        large.line_to(0.0, -0.0);
        let large = large.finish().unwrap();
        // IDs include implicit wrappers; raw bits distinguish negative zero.
        let signature = |path: &Path| {
            path.0
                .iter()
                .map(|data| {
                    (
                        data.id(),
                        data.args()
                            .map(|args| args.iter().map(|v| v.to_bits()).collect::<Vec<_>>()),
                    )
                })
                .collect::<Vec<_>>()
        };
        for paths in [
            vec![],
            vec![path.clone()],
            vec![path.clone(), path],
            vec![large],
        ] {
            for round in [
                1.0,
                10.0,
                100.0,
                1_000.0,
                1e12,
                10e12,
                1e40,
                f64::MAX,
                0.0,
                -1.0,
                f64::NAN,
                f64::INFINITY,
            ] {
                let direct = commands_from_paths(&paths, round);
                let mut text = String::new();
                for path in &paths {
                    append_path(&mut text, path, round);
                }
                let parsed = Path::parse_string(text.trim_end()).ok();
                match (direct, parsed) {
                    (Some(a), Some(b)) => {
                        assert_eq!(
                            signature(&a),
                            signature(&b),
                            "input differs at rounding {round}"
                        );
                        assert_eq!(
                            signature(&optimize_commands(a)),
                            signature(&optimize_commands(b)),
                            "optimized output differs at rounding {round}"
                        );
                    }
                    (None, None) => {}
                    _ => panic!("input acceptance differs at rounding {round}"),
                }
            }
        }
    }

    #[test]
    fn rounded_coordinate_matches_allocating_formatter() {
        let values = [
            -0.0,
            0.0,
            -1.5,
            0.005,
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
