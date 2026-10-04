//! Test-only extraction experiments, not the production color pipeline.
//! Lock both successful cases and counterexamples; green research tests do not
//! imply that every supported SVG can safely use root-marker extraction.
use usvg::{Color, Node, Options, Paint, Tree};

mod geometry;
mod preflight;

fn svg(root: &str, body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" {root}>{body}</svg>"#
    )
}

fn rect(attributes: &str) -> String {
    format!(r#"<path d="M10 10H90V90H10Z" {attributes}/>"#)
}

fn fills(tree: &Tree) -> Vec<(Color, f32)> {
    fn walk(group: &usvg::Group, out: &mut Vec<(Color, f32)>) {
        for node in group.children() {
            match node {
                Node::Group(group) => walk(group, out),
                Node::Path(path) => {
                    if let Some(fill) = path.fill()
                        && let Paint::Color(color) = fill.paint()
                    {
                        out.push((*color, fill.opacity().get()));
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = vec![];
    walk(tree.root(), &mut out);
    out
}

fn sentinel(root: &str, body: &str) -> (Color, Vec<(Color, f32)>) {
    let (marker, tree) = marker_tree(&svg(root, body));
    (marker, fills(&tree))
}

fn parse(source: &str) -> Tree {
    let document = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap();
    Tree::from_xmltree(&document, &Options::default()).unwrap()
}

fn marker_tree(source: &str) -> (Color, Tree) {
    let original = parse(source);
    let used = fills(&original);
    let sentinel = (0..=0xffffff)
        .map(|rgb| Color::new_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8))
        .find(|color| !used.iter().any(|(used, _)| used == color))
        .unwrap();
    (sentinel, parse(&inject_root_defaults(source, sentinel)))
}

// Insert attributes after the root QName, not by replacing arbitrary "<svg" text.
// Existing attributes remain intact; usvg resolves inline/stylesheet precedence.
fn inject_root_defaults(source: &str, marker: Color) -> String {
    let document = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap();
    let root = document.root_element();
    assert!(root.has_tag_name(("http://www.w3.org/2000/svg", "svg")));
    let start = root.range().start + 1;
    let end = start
        + source[start..]
            .find(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>'))
            .unwrap();
    let mut attributes = String::new();
    for name in ["fill", "color"] {
        if root.attribute(name).is_none() {
            attributes.push_str(&format!(
                r##" {name}="#{:02x}{:02x}{:02x}""##,
                marker.red, marker.green, marker.blue
            ));
        }
    }
    format!("{}{}{}", &source[..end], attributes, &source[end..])
}

#[test]
fn color_proof_sentinel_handles_initial_authored_and_opacity() {
    let red = Color::new_rgb(255, 0, 0);
    for (root, body, foreground, color, alpha) in [
        ("", rect(""), true, red, 1.0),
        ("", rect(r#"fill="currentColor""#), true, red, 1.0),
        (r#"fill="currentColor""#, rect(""), true, red, 1.0),
        ("", rect(r#"fill-opacity="0.25""#), true, red, 0.25),
        (r#"fill="red""#, rect(""), false, red, 1.0),
        (r#"style="fill:red""#, rect(""), false, red, 1.0),
        ("", rect(r#"fill="red""#), false, red, 1.0),
        ("", rect(r#"style="fill:red""#), false, red, 1.0),
        (
            "",
            format!(r#"<g fill="red" fill-opacity="0.25">{}</g>"#, rect("")),
            false,
            red,
            0.25,
        ),
        (
            "",
            format!("<style>path {{ fill: red }}</style>{}", rect("")),
            false,
            red,
            1.0,
        ),
        (
            "",
            format!("<style>svg {{ fill: red }}</style>{}", rect("")),
            false,
            red,
            1.0,
        ),
        (r#"fill="black""#, rect(""), false, Color::black(), 1.0),
    ] {
        let (marker, actual) = sentinel(root, &body);
        assert_eq!(
            actual,
            vec![(if foreground { marker } else { color }, alpha)],
            "{root} {body}"
        );
    }
    assert!(sentinel("", &rect(r#"fill="none""#)).1.is_empty());
}

#[test]
fn color_proof_current_color_respects_authored_color() {
    // Revised contract: authored effective color stays fixed, including when
    // currentColor is inherited. These were failures under the original plan.
    for (root, body) in [
        (r#"color="red""#, rect(r#"fill="currentColor""#)),
        (r#"color="red" fill="currentColor""#, rect("")),
        ("", rect(r#"color="red" fill="currentColor""#)),
        ("", rect(r#"style="color:red;fill:currentColor""#)),
        (
            "",
            format!(r#"<g color="red">{}</g>"#, rect(r#"fill="currentColor""#)),
        ),
        (
            "",
            format!(r#"<g color="red" fill="currentColor">{}</g>"#, rect("")),
        ),
        (
            "",
            format!(
                "<style>path {{ color: red; fill: currentColor }}</style>{}",
                rect("")
            ),
        ),
    ] {
        let (marker, actual) = sentinel(root, &body);
        assert_ne!(actual[0].0, marker);
        assert_eq!(actual, vec![(Color::new_rgb(255, 0, 0), 1.0)]);
    }
}

#[test]
fn color_proof_marker_cascade_references_and_order() {
    let red = Color::new_rgb(255, 0, 0);
    let blue = Color::new_rgb(0, 0, 255);
    for (root, body, expected) in [
        (r#"color="red""#, format!(r#"<g color="blue">{}</g>"#, rect(r#"fill="currentColor""#)), blue),
        (r#"fill="blue" style="fill : red""#, rect(""), red),
        (r#"style="fill:blue""#, format!("<style>svg {{ fill:red !important }}</style>{}", rect("")), red),
        ("", format!("<style>.paint {{fill:blue}} #chosen {{fill:red}}</style>{}", rect(r#"class="paint" id="chosen""#)), red),
        (r#"fill="red""#, rect(r#"fill="inherit""#), red),
        ("", r##"<defs><path id="shape" d="M10 10H90V90H10Z"/></defs><use href="#shape" fill="red"/>"##.into(), red),
    ] {
        let (marker, actual) = sentinel(root, &body);
        assert_ne!(expected, marker);
        assert_eq!(actual, vec![(expected, 1.0)], "{root} {body}");
    }
    let source = svg(
        "",
        &format!(
            r##"<defs><path id="shape" d="M10 10H90V90H10Z"/></defs><g transform="translate(3 4)"><use href="#shape"/>{}</g>"##,
            rect(r#"fill="red" fill-opacity="0.25""#)
        ),
    );
    let (marker, painted) = marker_tree(&source);
    assert_eq!(fills(&painted), vec![(marker, 1.0), (red, 0.25)]);
    fn geometry(group: &usvg::Group, out: &mut Vec<String>) {
        for node in group.children() {
            match node {
                Node::Group(child) => geometry(child, out),
                Node::Path(path) => {
                    out.push(format!("{:?} {:?}", path.data(), path.abs_transform()))
                }
                _ => {}
            }
        }
    }
    let mut before = vec![];
    let mut after = vec![];
    geometry(parse(&source).root(), &mut before);
    geometry(painted.root(), &mut after);
    assert_eq!(
        before, after,
        "paint extraction must not reorder or transform paths"
    );
}

#[test]
fn color_proof_root_rewrite_preserves_xml_and_avoids_color_collision() {
    let source = r##"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY red "#ff0000">]><!-- <svg fill='wrong'> --><s:svg xmlns:s="http://www.w3.org/2000/svg" width="100" height="100" data-note="a > b"><s:path id="paint" fill="&red;" d="M1 1H9V9H1Z"/><s:path fill="#000001" d="M20 20H40V40H20Z"/><s:path d="M50 50H90V90H50Z"/></s:svg>"##;
    let (marker, tree) = marker_tree(source);
    assert_eq!(marker, Color::new_rgb(0, 0, 2));
    assert_eq!(
        fills(&tree),
        vec![
            (Color::new_rgb(255, 0, 0), 1.0),
            (Color::new_rgb(0, 0, 1), 1.0),
            (marker, 1.0)
        ]
    );
    assert!(tree.node_by_id("paint").is_some());
    let rewritten = inject_root_defaults(source, marker);
    assert!(rewritten.contains(r##"<!ENTITY red "#ff0000">"##));
    assert!(rewritten.contains("<!-- <svg fill='wrong'> -->"));
    let empty = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>"#;
    assert!(marker_tree(empty).1.root().children().is_empty());
}

#[test]
fn color_proof_root_attributes_can_change_authored_selector_matching() {
    // A concrete counterexample to calling the marker approach universally safe.
    let source = svg(
        "",
        &format!("<style>svg[fill] path {{ fill:red }}</style>{}", rect("")),
    );
    assert_eq!(fills(&parse(&source)), vec![(Color::black(), 1.0)]);
    let (marker, painted) = marker_tree(&source);
    assert_ne!(marker, Color::new_rgb(255, 0, 0));
    assert_eq!(fills(&painted), vec![(Color::new_rgb(255, 0, 0), 1.0)]);
    // Injecting CSS instead is not automatically safer: it overrides an authored
    // root presentation attribute rather than acting as an SVG initial value.
    let options = Options {
        style_sheet: Some("svg {fill:#010203}".into()),
        ..Default::default()
    };
    let tree = Tree::from_str(&svg(r#"fill="red""#, &rect("")), &options).unwrap();
    assert_eq!(fills(&tree), vec![(Color::new_rgb(1, 2, 3), 1.0)]);
}

#[test]
fn color_proof_css_reset_observations() {
    for reset in ["initial", "inherit", "unset", "revert", "revert-layer"] {
        let (marker, actual) =
            sentinel(r#"fill="red""#, &rect(&format!(r#"style="fill:{reset}""#)));
        println!("CSS fill:{reset}: {actual:?}; sentinel={marker:?}");
        let expected = if reset == "inherit" {
            Color::new_rgb(255, 0, 0)
        } else {
            Color::black()
        };
        assert_eq!(actual, vec![(expected, 1.0)]);
        assert_ne!(actual[0].0, marker);
    }
}

#[test]
fn color_proof_unsupported_source_can_disappear_from_usvg_tree() {
    let baseline = Tree::from_str(&svg("", &rect("")), &Options::default()).unwrap();
    for extra in [
        r#"<foreignObject width="100" height="100"><div>unsupported</div></foreignObject>"#,
        r#"<image href="missing.png" width="100" height="100"/>"#,
        r#"<text x="0" y="50">unsupported</text>"#,
    ] {
        let source = svg("", &format!("{}{extra}", rect("")));
        let tree = Tree::from_str(&source, &Options::default()).unwrap();
        assert_eq!(
            format!("{:?}", tree.root()),
            format!("{:?}", baseline.root()),
            "{extra}"
        );
    }
    // An unresolved filter drops the path; an unresolved clip is discarded.
    // Selected-source validation needs preflight input to explain either.
    for attr in [r#"filter="url(#missing)""#, r#"clip-path="url(#missing)""#] {
        let tree = Tree::from_str(&svg("", &rect(attr)), &Options::default()).unwrap();
        if attr.starts_with("filter") {
            assert!(tree.root().children().is_empty());
        } else {
            assert_eq!(
                format!("{:?}", tree.root()),
                format!("{:?}", baseline.root())
            );
        }
    }
}
