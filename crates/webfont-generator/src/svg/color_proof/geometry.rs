//! Why existing monochrome normalization cannot be used for color layers.
use kurbo::{Point, Shape};

fn outlines(source: &str, rule: &str) -> (kurbo::BezPath, kurbo::BezPath) {
    let tree = super::parse(&super::svg(
        "",
        &format!(r#"<path d="{source}" fill-rule="{rule}"/>"#),
    ));
    let usvg::Node::Path(path) = &tree.root().children()[0] else {
        panic!("path expected")
    };
    assert_eq!(
        path.fill().unwrap().rule() == usvg::FillRule::EvenOdd,
        rule == "evenodd"
    );
    let original =
        crate::svg::geometry::rounded_bezpath_from_tiny_paths(&[path.data().clone()], 1000.0);
    let normalized = crate::svg::winding::normalize_winding(vec![path.data().clone()]);
    let fallback = crate::svg::geometry::rounded_bezpath_from_tiny_paths(&normalized, 1000.0);
    (original, fallback)
}

#[test]
fn color_proof_evenodd_overlap_requires_more_than_containment_reversal() {
    let (original, fallback) = outlines("M0 0H60V60H0Z M30 30H90V90H30Z", "evenodd");
    let overlap = Point::new(45.0, 45.0);
    assert_eq!(original.winding(overlap).abs() % 2, 0);
    assert_ne!(
        fallback.winding(overlap),
        0,
        "fallback incorrectly fills the evenodd overlap"
    );
}

#[test]
fn color_proof_nonzero_layer_must_bypass_monochrome_containment_heuristic() {
    let (original, fallback) = outlines("M0 0H100V100H0Z M20 20H80V80H20Z", "nonzero");
    assert_ne!(original.winding(Point::new(50.0, 50.0)), 0);
    assert_eq!(fallback.winding(Point::new(50.0, 50.0)), 0);
    // Existing behavior remains intentional for fallback, but is wrong for a
    // nonzero color layer, even when applied to only one path at a time.
}

#[test]
fn color_proof_nested_evenodd_holes_and_self_intersection_are_distinct_cases() {
    let (original, normalized) = outlines("M0 0H100V100H0Z M20 20H80V80H20Z", "evenodd");
    for (point, filled) in [
        (Point::new(10.0, 10.0), true),
        (Point::new(50.0, 50.0), false),
    ] {
        assert_eq!(original.winding(point).abs() % 2 == 1, filled);
        assert_eq!(normalized.winding(point) != 0, filled);
    }
    let (bowtie, _) = outlines("M0 0L100 100L0 100L100 0Z", "evenodd");
    assert_eq!(bowtie.winding(Point::new(50.0, 10.0)).abs() % 2, 1);
    assert_eq!(bowtie.winding(Point::new(10.0, 50.0)), 0);
}
