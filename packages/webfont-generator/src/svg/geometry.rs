use std::hash::Hasher;

use kurbo::{BezPath, PathEl, Point};
use rustc_hash::FxHasher;
use usvg::tiny_skia_path::{Path as TinyPath, PathSegment};

use super::rounded_coordinate;

pub(crate) fn bezpath_from_oxvg_path(path: &oxvg_path::Path) -> BezPath {
    let number = |value: f64| if value == 0.0 { 0.0 } else { value };
    let point = |x: f64, y: f64| Point::new(number(x), number(y));
    let relative = |from: Point, x: f64, y: f64| from + point(x, y).to_vec2();
    let mut result = BezPath::new();
    let mut current = Point::ORIGIN;
    let mut subpath_start = Point::ORIGIN;
    // Preserve Kurbo's family-agnostic smooth-control state for exact historical TTF bytes.
    let mut last_ctrl = None;
    let mut implicit_moveto = None;

    for command in &path.0 {
        let mut command = command;
        while let oxvg_path::command::Data::Implicit(inner) = command {
            command = inner;
        }

        if !matches!(
            command,
            oxvg_path::command::Data::MoveTo(_) | oxvg_path::command::Data::MoveBy(_)
        ) && let Some(to) = implicit_moveto.take()
        {
            result.move_to(to);
        }

        use oxvg_path::command::Data;
        match command {
            Data::MoveTo([x, y]) => {
                implicit_moveto = None;
                current = point(*x, *y);
                result.move_to(current);
                subpath_start = current;
                last_ctrl = Some(current);
            }
            Data::MoveBy([x, y]) => {
                implicit_moveto = None;
                current = relative(current, *x, *y);
                result.move_to(current);
                subpath_start = current;
                last_ctrl = Some(current);
            }
            Data::ClosePath => {
                result.close_path();
                current = subpath_start;
                implicit_moveto = Some(subpath_start);
            }
            Data::LineTo([x, y]) => {
                current = point(*x, *y);
                result.line_to(current);
                last_ctrl = Some(current);
            }
            Data::LineBy([x, y]) => {
                current = relative(current, *x, *y);
                result.line_to(current);
                last_ctrl = Some(current);
            }
            Data::HorizontalLineTo([x]) => {
                current = Point::new(number(*x), current.y);
                result.line_to(current);
                last_ctrl = Some(current);
            }
            Data::HorizontalLineBy([x]) => {
                current = Point::new(current.x + number(*x), current.y);
                result.line_to(current);
                last_ctrl = Some(current);
            }
            Data::VerticalLineTo([y]) => {
                current = Point::new(current.x, number(*y));
                result.line_to(current);
                last_ctrl = Some(current);
            }
            Data::VerticalLineBy([y]) => {
                current = Point::new(current.x, current.y + number(*y));
                result.line_to(current);
                last_ctrl = Some(current);
            }
            Data::CubicBezierTo([x1, y1, x2, y2, x, y]) => {
                let control1 = point(*x1, *y1);
                let control2 = point(*x2, *y2);
                current = point(*x, *y);
                result.curve_to(control1, control2, current);
                last_ctrl = Some(control2);
            }
            Data::CubicBezierBy([x1, y1, x2, y2, x, y]) => {
                let control1 = relative(current, *x1, *y1);
                let control2 = relative(current, *x2, *y2);
                current = relative(current, *x, *y);
                result.curve_to(control1, control2, current);
                last_ctrl = Some(control2);
            }
            Data::SmoothBezierTo([x2, y2, x, y]) => {
                let control1 = last_ctrl
                    .map(|control| (2.0 * current.to_vec2() - control.to_vec2()).to_point())
                    .unwrap_or(current);
                let control2 = point(*x2, *y2);
                current = point(*x, *y);
                result.curve_to(control1, control2, current);
                last_ctrl = Some(control2);
            }
            Data::SmoothBezierBy([x2, y2, x, y]) => {
                let control1 = last_ctrl
                    .map(|control| (2.0 * current.to_vec2() - control.to_vec2()).to_point())
                    .unwrap_or(current);
                let control2 = relative(current, *x2, *y2);
                current = relative(current, *x, *y);
                result.curve_to(control1, control2, current);
                last_ctrl = Some(control2);
            }
            Data::QuadraticBezierTo([x1, y1, x, y]) => {
                let control = point(*x1, *y1);
                current = point(*x, *y);
                result.quad_to(control, current);
                last_ctrl = Some(control);
            }
            Data::QuadraticBezierBy([x1, y1, x, y]) => {
                let control = relative(current, *x1, *y1);
                current = relative(current, *x, *y);
                result.quad_to(control, current);
                last_ctrl = Some(control);
            }
            Data::SmoothQuadraticBezierTo([x, y]) => {
                let control = last_ctrl
                    .map(|control| (2.0 * current.to_vec2() - control.to_vec2()).to_point())
                    .unwrap_or(current);
                current = point(*x, *y);
                result.quad_to(control, current);
                last_ctrl = Some(control);
            }
            Data::SmoothQuadraticBezierBy([x, y]) => {
                let control = last_ctrl
                    .map(|control| (2.0 * current.to_vec2() - control.to_vec2()).to_point())
                    .unwrap_or(current);
                current = relative(current, *x, *y);
                result.quad_to(control, current);
                last_ctrl = Some(control);
            }
            Data::ArcTo([rx, ry, rotation, large_arc, sweep, x, y])
            | Data::ArcBy([rx, ry, rotation, large_arc, sweep, x, y]) => {
                let to = if matches!(command, Data::ArcBy(_)) {
                    relative(current, *x, *y)
                } else {
                    point(*x, *y)
                };
                let svg_arc = kurbo::SvgArc {
                    from: current,
                    to,
                    radii: kurbo::Vec2::new(number(*rx), number(*ry)),
                    x_rotation: number(*rotation).to_radians(),
                    large_arc: number(*large_arc) == 1.0,
                    sweep: number(*sweep) == 1.0,
                };
                if let Some(arc) = kurbo::Arc::from_svg_arc(&svg_arc) {
                    arc.to_cubic_beziers(0.1, |p1, p2, p3| result.curve_to(p1, p2, p3));
                } else {
                    result.line_to(to);
                }
                current = to;
                last_ctrl = Some(to);
            }
            Data::Implicit(_) => unreachable!(),
        }
    }

    result
}

pub(crate) fn rounded_bezpath_from_tiny_paths(paths: &[TinyPath], round: f64) -> BezPath {
    let point = |point: usvg::tiny_skia_path::Point| {
        Point::new(
            rounded_coordinate(point.x, round),
            rounded_coordinate(point.y, round),
        )
    };
    let mut result = BezPath::new();
    for path in paths {
        for segment in path.segments() {
            match segment {
                PathSegment::MoveTo(to) => result.move_to(point(to)),
                PathSegment::LineTo(to) => result.line_to(point(to)),
                PathSegment::QuadTo(control, to) => result.quad_to(point(control), point(to)),
                PathSegment::CubicTo(control1, control2, to) => {
                    result.curve_to(point(control1), point(control2), point(to));
                }
                PathSegment::Close => result.close_path(),
            }
        }
    }
    result
}

pub(crate) fn bezpath_hash(path: &BezPath) -> u64 {
    let mut hasher = FxHasher::default();
    for element in path.elements() {
        hash_path_element(&mut hasher, *element);
    }
    hasher.finish()
}

fn hash_path_element(hasher: &mut FxHasher, element: PathEl) {
    match element {
        PathEl::MoveTo(point) => {
            hasher.write_u8(0);
            hash_point(hasher, point);
        }
        PathEl::LineTo(point) => {
            hasher.write_u8(1);
            hash_point(hasher, point);
        }
        PathEl::QuadTo(control, point) => {
            hasher.write_u8(2);
            hash_point(hasher, control);
            hash_point(hasher, point);
        }
        PathEl::CurveTo(control1, control2, point) => {
            hasher.write_u8(3);
            hash_point(hasher, control1);
            hash_point(hasher, control2);
            hash_point(hasher, point);
        }
        PathEl::ClosePath => hasher.write_u8(4),
    }
}

fn hash_point(hasher: &mut FxHasher, point: Point) {
    hasher.write_u64(point.x.to_bits());
    hasher.write_u64(point.y.to_bits());
}
