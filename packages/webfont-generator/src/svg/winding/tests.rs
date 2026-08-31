use super::*;

/// Signed-area sign of every contour across the given paths (after flattening).
fn signs(paths: &[TinyPath]) -> Vec<i8> {
    let mut out = Vec::new();
    for path in paths {
        let mut contours = Vec::new();
        decompose(path, &mut contours);
        for mut c in contours {
            flatten_contour(&mut c);
            out.push(sign(c.area2));
        }
    }
    out
}

/// Append an axis-aligned square as one subpath, wound in a fixed direction.
fn square(b: &mut PathBuilder, x: f32, y: f32, s: f32) {
    b.move_to(x, y);
    b.line_to(x, y + s);
    b.line_to(x + s, y + s);
    b.line_to(x + s, y);
    b.close();
}

/// Exact segment sequence (verb + coordinates) across all paths — for byte-preservation checks.
fn segment_repr(paths: &[TinyPath]) -> Vec<String> {
    let mut out = Vec::new();
    for path in paths {
        for seg in path.segments() {
            out.push(match seg {
                PathSegment::MoveTo(p) => format!("M {} {}", p.x, p.y),
                PathSegment::LineTo(p) => format!("L {} {}", p.x, p.y),
                PathSegment::QuadTo(c, p) => format!("Q {} {} {} {}", c.x, c.y, p.x, p.y),
                PathSegment::CubicTo(a, b, p) => {
                    format!("C {} {} {} {} {} {}", a.x, a.y, b.x, b.y, p.x, p.y)
                }
                PathSegment::Close => "Z".to_string(),
            });
        }
    }
    out
}

#[test]
fn nested_same_wound_contour_is_reversed_into_a_hole() {
    let mut b = PathBuilder::new();
    square(&mut b, 0.0, 0.0, 30.0); // outer
    square(&mut b, 10.0, 10.0, 10.0); // inner, contained, same winding
    let out = normalize_winding(vec![b.finish().unwrap()]);
    let s = signs(&out);
    assert_eq!(s.len(), 2);
    assert_ne!(
        s[0], s[1],
        "a contained same-wound contour must become an opposite-wound hole"
    );
}

#[test]
fn overlapping_but_unnested_contours_stay_unioned() {
    let mut b = PathBuilder::new();
    square(&mut b, 0.0, 0.0, 30.0); // A
    square(&mut b, 20.0, 20.0, 30.0); // B overlaps A but extends beyond it -> not contained
    let out = normalize_winding(vec![b.finish().unwrap()]);
    let s = signs(&out);
    assert_eq!(s.len(), 2);
    assert_eq!(
        s[0], s[1],
        "overlapping non-nested contours must stay same-wound (union, not a hole)"
    );
}

#[test]
fn concave_outer_rejects_inner_edges_that_cross_outside() {
    let mut b = PathBuilder::new();
    // U-shaped outer: all four corners of the inner rectangle are inside the two arms, but its
    // horizontal edges bridge across the open notch, so it is overlapping rather than nested.
    b.move_to(0.0, 0.0);
    b.line_to(0.0, 30.0);
    b.line_to(10.0, 30.0);
    b.line_to(10.0, 10.0);
    b.line_to(20.0, 10.0);
    b.line_to(20.0, 30.0);
    b.line_to(30.0, 30.0);
    b.line_to(30.0, 0.0);
    b.close();
    b.move_to(5.0, 15.0);
    b.line_to(5.0, 25.0);
    b.line_to(25.0, 25.0);
    b.line_to(25.0, 15.0);
    b.close();
    let out = normalize_winding(vec![b.finish().unwrap()]);
    let s = signs(&out);
    assert_eq!(s.len(), 2);
    assert_eq!(s[0], s[1], "crossing edges must keep the contours unioned");
}

#[test]
fn already_correct_hole_is_left_unchanged() {
    let mut b = PathBuilder::new();
    // outer
    b.move_to(0.0, 0.0);
    b.line_to(0.0, 30.0);
    b.line_to(30.0, 30.0);
    b.line_to(30.0, 0.0);
    b.close();
    // inner, wound the OPPOSITE way (already a proper hole)
    b.move_to(10.0, 10.0);
    b.line_to(20.0, 10.0);
    b.line_to(20.0, 20.0);
    b.line_to(10.0, 20.0);
    b.close();
    let input = b.finish().unwrap();
    let before = signs(std::slice::from_ref(&input));
    let after = signs(&normalize_winding(vec![input]));
    assert_eq!(before, after, "an already-correct hole must be idempotent");
    assert_ne!(after[0], after[1]);
}

#[test]
fn single_contour_is_untouched() {
    assert!(normalize_winding(Vec::new()).is_empty());

    let mut b = PathBuilder::new();
    square(&mut b, 0.0, 0.0, 30.0);
    let input = b.finish().unwrap();
    assert_eq!(
        signs(&normalize_winding(vec![input.clone()])),
        signs(std::slice::from_ref(&input))
    );
}

// Glyphs that need no reversal must come out byte-identical (the module promises this), so assert
// the exact segment sequence is preserved — not merely the winding signs.
#[test]
fn noop_inputs_preserve_exact_segments() {
    // single contour
    let mut single = PathBuilder::new();
    square(&mut single, 0.0, 0.0, 30.0);
    // open contour
    let mut open = PathBuilder::new();
    open.move_to(0.0, 0.0);
    open.line_to(30.0, 30.0);
    // move-only subpath followed by one real contour
    let mut move_only = PathBuilder::new();
    move_only.move_to(50.0, 50.0);
    square(&mut move_only, 0.0, 0.0, 30.0);
    // overlapping, non-nested
    let mut overlap = PathBuilder::new();
    square(&mut overlap, 0.0, 0.0, 30.0);
    square(&mut overlap, 20.0, 20.0, 30.0);
    // already-correct opposite-wound hole (outer CW, inner CCW)
    let mut hole = PathBuilder::new();
    hole.move_to(0.0, 0.0);
    hole.line_to(0.0, 30.0);
    hole.line_to(30.0, 30.0);
    hole.line_to(30.0, 0.0);
    hole.close();
    hole.move_to(10.0, 10.0);
    hole.line_to(20.0, 10.0);
    hole.line_to(20.0, 20.0);
    hole.line_to(10.0, 20.0);
    hole.close();

    for input in [
        single.finish().unwrap(),
        open.finish().unwrap(),
        move_only.finish().unwrap(),
        overlap.finish().unwrap(),
        hole.finish().unwrap(),
    ] {
        let before = segment_repr(std::slice::from_ref(&input));
        let after = segment_repr(&normalize_winding(vec![input]));
        assert_eq!(
            before, after,
            "a glyph needing no reversal must be returned byte-identical"
        );
    }
}

#[test]
fn nested_contours_across_paths_are_normalized() {
    let mut outer = PathBuilder::new();
    square(&mut outer, 0.0, 0.0, 30.0);
    let mut inner = PathBuilder::new();
    square(&mut inner, 10.0, 10.0, 10.0);

    let signs = signs(&normalize_winding(vec![
        outer.finish().unwrap(),
        inner.finish().unwrap(),
    ]));
    assert_eq!(signs.len(), 2);
    assert_ne!(signs[0], signs[1]);
}

// Containment is resolved past depth 1: four nested squares (same winding) must come out strictly
// alternating, so each level reads as fill / hole / fill / hole.
#[test]
fn multi_level_nesting_alternates() {
    let mut b = PathBuilder::new();
    square(&mut b, 0.0, 0.0, 40.0);
    square(&mut b, 5.0, 5.0, 30.0);
    square(&mut b, 10.0, 10.0, 20.0);
    square(&mut b, 15.0, 15.0, 10.0);
    let s = signs(&normalize_winding(vec![b.finish().unwrap()]));
    assert_eq!(s.len(), 4);
    assert_ne!(s[0], s[1], "depth 0 vs 1");
    assert_ne!(s[1], s[2], "depth 1 vs 2");
    assert_ne!(s[2], s[3], "depth 2 vs 3");
    assert_eq!(s[0], s[2], "even depths share orientation");
    assert_eq!(s[1], s[3], "odd depths share orientation");
}
