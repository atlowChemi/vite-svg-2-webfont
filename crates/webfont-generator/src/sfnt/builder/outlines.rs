use std::io::{Error, ErrorKind};

use kurbo::{BezPath, CubicBez, PathEl, Point};

const QUAD_APPROXIMATION_ACCURACY: f64 = 0.25;
pub(super) const SIMPLIFY_TOLERANCE: f64 = 0.3;

pub(super) fn quadratic_path_from_svg_path_data(path_data: &str) -> Result<BezPath, Error> {
    let path = BezPath::from_svg(path_data).map_err(|error| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Failed to parse generated SVG path data as a Bezier path: {error:?}"),
        )
    })?;
    quadratic_path(&path)
}

pub(super) fn quadratic_path(path: &BezPath) -> Result<BezPath, Error> {
    let mut elements: Vec<PathEl> = Vec::with_capacity(path.elements().len());
    let mut current: Option<Point> = None;
    let mut line_start: Option<Point> = None;

    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                elements.push(PathEl::MoveTo(point));
                current = Some(point);
                line_start = None;
            }
            PathEl::LineTo(point) => {
                let from = current.unwrap_or(point);
                current = Some(push_line(&mut elements, &mut line_start, from, point));
            }
            PathEl::QuadTo(control, point) => {
                let from = current.unwrap_or(control);
                if point_line_distance(control, from, point) <= SIMPLIFY_TOLERANCE {
                    current = Some(push_line(&mut elements, &mut line_start, from, point));
                } else {
                    elements.push(PathEl::QuadTo(control, point));
                    line_start = None;
                    current = Some(point);
                }
            }
            PathEl::CurveTo(control1, control2, point) => {
                let start = current.ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidData,
                        "Encountered a cubic segment before a MoveTo while building a TTF glyph.",
                    )
                })?;
                let cubic = CubicBez::new(start, control1, control2, point);
                let mut from = start;
                for (_, _, quad) in cubic.to_quads(QUAD_APPROXIMATION_ACCURACY) {
                    if point_line_distance(quad.p1, from, quad.p2) <= SIMPLIFY_TOLERANCE {
                        from = push_line(&mut elements, &mut line_start, from, quad.p2);
                    } else {
                        elements.push(PathEl::QuadTo(quad.p1, quad.p2));
                        line_start = None;
                        from = quad.p2;
                    }
                }
                current = Some(from);
            }
            PathEl::ClosePath => {
                elements.push(PathEl::ClosePath);
                current = None;
                line_start = None;
            }
        }
    }
    Ok(BezPath::from_vec(elements))
}

pub(super) fn point_line_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f64::EPSILON {
        return (p - a).hypot();
    }
    ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / len_sq.sqrt()
}

fn push_line(
    elements: &mut Vec<PathEl>,
    line_start: &mut Option<Point>,
    from: Point,
    point: Point,
) -> Point {
    if let Some(start) = *line_start
        && point_line_distance(from, start, point) <= SIMPLIFY_TOLERANCE
        && let Some(PathEl::LineTo(end)) = elements.last_mut()
    {
        *end = point;
        return point;
    }
    elements.push(PathEl::LineTo(point));
    *line_start = Some(from);
    point
}
