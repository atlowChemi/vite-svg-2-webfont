//! Contour-winding normalization.
//!
//! A glyph is filled with TrueType's nonzero rule, and a monochrome icon glyph collapses a possibly
//! multi-colour source to a single fill. This pass applies a **geometric heuristic**: a contour
//! fully contained in another is wound opposite to it, so nested contours alternate and become
//! holes (as svg2ttf/fontello do). Only contours whose winding *disagrees* with their nesting are
//! reversed, so an already-correctly-wound glyph (the common case, incl. stroke outlines) is
//! returned untouched and byte-identical.
//!
//! This is intentionally **not** a fill-rule-faithful flatten — it ignores each path's `fill-rule`
//! and infers holes purely from containment. Deliberate choice: the spec-correct flatten (honour
//! `fill-rule`, union across paths) renders the dominant real-world pattern — a foreground shape
//! over a coloured background — as a solid blob, because the foreground unions into the background
//! instead of reading as a knockout. The containment heuristic matches that intent far better in
//! practice. Trade-off: a genuinely-intended *filled* nested region of the same paint (rare) would
//! be turned into a hole. Validated visually across a large real icon set.

use usvg::tiny_skia_path::{Path as TinyPath, PathBuilder, PathSegment, PathVerb, Point};

// Curves are flattened only to decide nesting + winding sign, so a coarse subdivision is plenty.
const FLATTEN_STEPS: usize = 6;
const GEOMETRY_EPSILON: f64 = 1e-9;

enum Step {
    Line(Point),
    Quad(Point, Point),
    Cubic(Point, Point, Point),
}

struct Contour {
    start: Point,
    steps: Vec<Step>,
    poly: Vec<Point>,
    area2: f64, // 2× signed area; sign = winding, magnitude for nesting comparison
    convex: bool,
    min: Point,
    max: Point,
}

/// Reverse-wind contours so that nested ones alternate orientation (become holes under nonzero).
/// Returns the input untouched when nothing needs flipping.
pub(crate) fn normalize_winding(paths: Vec<TinyPath>) -> Vec<TinyPath> {
    let single_contour = paths
        .iter()
        .flat_map(|path| path.verbs())
        .filter(|verb| **verb == PathVerb::Move)
        .take(2)
        .count()
        < 2;
    if single_contour {
        return paths;
    }

    let mut contours: Vec<Contour> = Vec::new();
    for path in &paths {
        decompose(path, &mut contours);
    }
    if contours.len() < 2 {
        return paths; // a single contour can't contain another — skip all flattening
    }

    // Only now (multiple contours) flatten curves to compute area, bbox, and the containment polys.
    for contour in &mut contours {
        flatten_contour(contour);
    }

    // Process largest-area first so a contour's container is resolved before it is.
    let mut order: Vec<usize> = (0..contours.len()).collect();
    order.sort_by(|&a, &b| contours[b].area2.abs().total_cmp(&contours[a].area2.abs()));

    let mut final_sign = vec![0i8; contours.len()];
    let mut reverse = vec![false; contours.len()];
    let mut any = false;
    for &i in &order {
        let parent = immediate_parent(&contours, i);
        let want = match parent {
            Some(p) => -final_sign[p],
            None => sign(contours[i].area2),
        };
        final_sign[i] = want;
        if want != sign(contours[i].area2) {
            reverse[i] = true;
            any = true;
        }
    }
    if !any {
        return paths;
    }

    // Rebuild every contour (in original order) into one path, reversing the flagged ones.
    let mut builder = PathBuilder::new();
    for (i, contour) in contours.iter().enumerate() {
        if reverse[i] {
            emit_reversed(&mut builder, contour);
        } else {
            emit_forward(&mut builder, contour);
        }
    }
    match builder.finish() {
        Some(path) => vec![path],
        None => paths,
    }
}

fn sign(area2: f64) -> i8 {
    if area2 >= 0.0 { 1 } else { -1 }
}

fn decompose(path: &TinyPath, out: &mut Vec<Contour>) {
    let mut start = Point::zero();
    let mut steps: Vec<Step> = Vec::new();
    let flush = |start: Point, steps: &mut Vec<Step>, out: &mut Vec<Contour>| {
        if steps.is_empty() {
            return;
        }
        out.push(Contour {
            start,
            steps: std::mem::take(steps),
            poly: Vec::new(),
            area2: 0.0,
            convex: true,
            min: Point::zero(),
            max: Point::zero(),
        });
    };
    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                flush(start, &mut steps, out);
                start = p;
            }
            PathSegment::LineTo(p) => steps.push(Step::Line(p)),
            PathSegment::QuadTo(c, p) => steps.push(Step::Quad(c, p)),
            PathSegment::CubicTo(c1, c2, p) => steps.push(Step::Cubic(c1, c2, p)),
            PathSegment::Close => flush(start, &mut steps, out),
        }
    }
    flush(start, &mut steps, out);
}

/// Flatten a contour's curves into its `poly` and compute its signed area + bbox (lazy: done only
/// for glyphs with multiple contours, where nesting is possible).
fn flatten_contour(c: &mut Contour) {
    let mut poly = Vec::with_capacity(c.steps.len() + 1);
    let mut cur = c.start;
    poly.push(cur);
    for step in &c.steps {
        match *step {
            Step::Line(p) => poly.push(p),
            Step::Quad(ctrl, p) => flatten_quad(cur, ctrl, p, &mut poly),
            Step::Cubic(c1, c2, p) => flatten_cubic(cur, c1, c2, p, &mut poly),
        }
        cur = match *step {
            Step::Line(p) | Step::Quad(_, p) | Step::Cubic(_, _, p) => p,
        };
    }
    c.area2 = shoelace(&poly);
    c.convex = is_convex(&poly);
    let (min, max) = bounds(&poly);
    c.min = min;
    c.max = max;
    c.poly = poly;
}

fn emit_forward(builder: &mut PathBuilder, c: &Contour) {
    builder.move_to(c.start.x, c.start.y);
    for step in &c.steps {
        match *step {
            Step::Line(p) => builder.line_to(p.x, p.y),
            Step::Quad(ctrl, p) => builder.quad_to(ctrl.x, ctrl.y, p.x, p.y),
            Step::Cubic(c1, c2, p) => builder.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y),
        }
    }
    builder.close();
}

fn emit_reversed(builder: &mut PathBuilder, c: &Contour) {
    // Endpoints in order: start, steps[0].end, steps[1].end, …
    let end_of = |k: usize| match c.steps[k] {
        Step::Line(p) | Step::Quad(_, p) | Step::Cubic(_, _, p) => p,
    };
    let start_of = |k: usize| if k == 0 { c.start } else { end_of(k - 1) };
    let last = end_of(c.steps.len() - 1);
    builder.move_to(last.x, last.y);
    for k in (0..c.steps.len()).rev() {
        let to = start_of(k);
        match c.steps[k] {
            Step::Line(_) => builder.line_to(to.x, to.y),
            Step::Quad(ctrl, _) => builder.quad_to(ctrl.x, ctrl.y, to.x, to.y),
            Step::Cubic(c1, c2, _) => builder.cubic_to(c2.x, c2.y, c1.x, c1.y, to.x, to.y),
        }
    }
    builder.close();
}

/// The smallest-area contour that fully contains contour `i`.
fn immediate_parent(contours: &[Contour], i: usize) -> Option<usize> {
    let mine = contours[i].area2.abs();
    let mut best: Option<usize> = None;
    for (j, other) in contours.iter().enumerate() {
        if j == i || other.area2.abs() <= mine {
            continue;
        }
        if contains(other, &contours[i])
            && best.is_none_or(|b| other.area2.abs() < contours[b].area2.abs())
        {
            best = Some(j);
        }
    }
    best
}

/// True only when `inner` is *entirely* inside `outer`: bbox-contained, every vertex inside, and no
/// edge crosses the outer boundary. Requiring full containment keeps overlapping-but-not-nested
/// contours (e.g. a padlock shackle resting on its body) from being mistaken for a hole.
fn contains(outer: &Contour, inner: &Contour) -> bool {
    if inner.min.x < outer.min.x
        || inner.min.y < outer.min.y
        || inner.max.x > outer.max.x
        || inner.max.y > outer.max.y
    {
        return false;
    }
    if !inner.poly.iter().all(|&p| point_in_polygon(p, &outer.poly)) {
        return false;
    }
    if outer.convex {
        return true;
    }
    !edges(&inner.poly).any(|inner_edge| {
        edges(&outer.poly).any(|outer_edge| {
            segments_intersect(inner_edge.0, inner_edge.1, outer_edge.0, outer_edge.1)
        })
    })
}

fn edges(poly: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    poly.iter()
        .copied()
        .zip(poly.iter().copied().cycle().skip(1))
        .take(poly.len())
}

fn shoelace(poly: &[Point]) -> f64 {
    let mut a = 0.0;
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        a += f64::from(p.x) * f64::from(q.y) - f64::from(q.x) * f64::from(p.y);
    }
    a
}

fn bounds(poly: &[Point]) -> (Point, Point) {
    let mut min = poly[0];
    let mut max = poly[0];
    for &p in poly {
        min = Point::from_xy(min.x.min(p.x), min.y.min(p.y));
        max = Point::from_xy(max.x.max(p.x), max.y.max(p.y));
    }
    (min, max)
}

fn is_convex(poly: &[Point]) -> bool {
    let mut direction = 0i8;
    for i in 0..poly.len() {
        let turn = orient(
            poly[i],
            poly[(i + 1) % poly.len()],
            poly[(i + 2) % poly.len()],
        );
        if turn.abs() <= GEOMETRY_EPSILON {
            continue;
        }
        let sign = if turn > 0.0 { 1 } else { -1 };
        if direction == 0 {
            direction = sign;
        } else if direction != sign {
            return false;
        }
    }
    true
}

fn point_in_polygon(pt: Point, poly: &[Point]) -> bool {
    let (px, py) = (f64::from(pt.x), f64::from(pt.y));
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (f64::from(poly[i].x), f64::from(poly[i].y));
        let (xj, yj) = (f64::from(poly[j].x), f64::from(poly[j].y));
        if (yi > py) != (yj > py) {
            let x_cross = (xj - xi) * (py - yi) / (yj - yi) + xi;
            if px < x_cross {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if o1.abs() <= GEOMETRY_EPSILON && on_segment(a, c, b)
        || o2.abs() <= GEOMETRY_EPSILON && on_segment(a, d, b)
        || o3.abs() <= GEOMETRY_EPSILON && on_segment(c, a, d)
        || o4.abs() <= GEOMETRY_EPSILON && on_segment(c, b, d)
    {
        return true;
    }
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn orient(a: Point, b: Point, c: Point) -> f64 {
    (f64::from(b.x) - f64::from(a.x)) * (f64::from(c.y) - f64::from(a.y))
        - (f64::from(b.y) - f64::from(a.y)) * (f64::from(c.x) - f64::from(a.x))
}

fn on_segment(a: Point, p: Point, b: Point) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

fn flatten_quad(p0: Point, c: Point, p1: Point, out: &mut Vec<Point>) {
    for k in 1..=FLATTEN_STEPS {
        let t = k as f32 / FLATTEN_STEPS as f32;
        let mt = 1.0 - t;
        out.push(Point::from_xy(
            mt * mt * p0.x + 2.0 * mt * t * c.x + t * t * p1.x,
            mt * mt * p0.y + 2.0 * mt * t * c.y + t * t * p1.y,
        ));
    }
}

fn flatten_cubic(p0: Point, c1: Point, c2: Point, p1: Point, out: &mut Vec<Point>) {
    for k in 1..=FLATTEN_STEPS {
        let t = k as f32 / FLATTEN_STEPS as f32;
        let mt = 1.0 - t;
        out.push(Point::from_xy(
            mt * mt * mt * p0.x
                + 3.0 * mt * mt * t * c1.x
                + 3.0 * mt * t * t * c2.x
                + t * t * t * p1.x,
            mt * mt * mt * p0.y
                + 3.0 * mt * mt * t * c1.y
                + 3.0 * mt * t * t * c2.y
                + t * t * t * p1.y,
        ));
    }
}

#[cfg(test)]
mod tests;
