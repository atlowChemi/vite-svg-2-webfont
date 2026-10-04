//! Minimal source-reachability experiment. This is NOT the production validator:
//! stylesheet cascade, visibility, and full error context remain integration work.
use std::collections::HashSet;

fn structural_features(source: &str) -> Vec<String> {
    let doc = roxmltree::Document::parse(source).unwrap();
    fn walk(
        node: roxmltree::Node<'_, '_>,
        references: &mut HashSet<roxmltree::NodeId>,
        out: &mut Vec<String>,
        referenced: bool,
    ) {
        let tag = node.tag_name().name();
        if !referenced
            && matches!(
                tag,
                "defs"
                    | "symbol"
                    | "linearGradient"
                    | "radialGradient"
                    | "pattern"
                    | "clipPath"
                    | "mask"
                    | "filter"
            )
        {
            return;
        }
        if matches!(tag, "foreignObject" | "text" | "image") {
            out.push(tag.into());
            return;
        }
        for name in ["stroke", "opacity", "clip-path", "mask", "filter"] {
            if let Some(value) = node.attribute(name)
                && value != "none"
            {
                out.push(format!("{name}={value}"));
            }
        }
        if let Some(value) = node.attribute("fill")
            && value.starts_with("url(")
        {
            out.push(format!("fill={value}"));
        }
        if tag == "use" {
            if let Some(href) = node
                .attribute("href")
                .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
            {
                if let Some(id) = href.strip_prefix('#') {
                    if let Some(target) = node
                        .document()
                        .descendants()
                        .find(|n| n.attribute("id") == Some(id))
                    {
                        if references.insert(target.id()) {
                            walk(target, references, out, true);
                            references.remove(&target.id());
                        } else {
                            out.push("cyclic use".into());
                        }
                    } else {
                        out.push("unresolved use".into());
                    }
                } else {
                    out.push("external use".into());
                }
            }
        }
        for child in node.children().filter(roxmltree::Node::is_element) {
            walk(child, references, out, false);
        }
    }
    let mut out = vec![];
    walk(doc.root_element(), &mut HashSet::new(), &mut out, false);
    out
}

#[test]
fn color_proof_preflight_finds_dropped_input_and_only_reachable_definitions() {
    let source = super::svg(
        "",
        r##"<defs><g id="unused"><image href="missing.png"/></g><g id="used"><text>unsupported</text></g></defs><path d="M0 0H10V10Z" filter="url(#missing)"/><use href="#used"/><foreignObject/><image href="missing.png"/>"##,
    );
    assert_eq!(
        structural_features(&source),
        ["filter=url(#missing)", "text", "foreignObject", "image"]
    );
    let cycles = super::svg(
        "",
        r##"<defs><g id="a"><use href="#a"/></g></defs><use href="#a"/><use href="#missing"/><use href="other.svg#shape"/>"##,
    );
    assert_eq!(
        structural_features(&cycles),
        ["cyclic use", "unresolved use", "external use"]
    );
}

#[test]
fn color_proof_preflight_classifies_each_attribute_category_in_source_order() {
    for feature in [
        "stroke=red",
        "opacity=0.5",
        "clip-path=url(#c)",
        "mask=url(#m)",
        "filter=url(#f)",
        "fill=url(#gradient)",
    ] {
        let (name, value) = feature.split_once('=').unwrap();
        let source = super::svg("", &super::rect(&format!(r#"{name}="{value}""#)));
        assert_eq!(structural_features(&source), [feature]);
    }
}

#[test]
fn color_proof_css_requires_preflight_even_when_normalized_tree_loses_effects() {
    let source = super::svg(
        "",
        &format!(
            "<style>path {{filter:url(#missing)}}</style>{}",
            super::rect("")
        ),
    );
    assert!(super::parse(&source).root().children().is_empty());
    // Mark the limit of this structural prototype rather than silently claiming
    // CSS declarations are validated. Source CSS must be inspected separately.
    assert!(structural_features(&source).is_empty());
    let document = roxmltree::Document::parse(&source).unwrap();
    assert!(
        document.descendants().any(|n| n.has_tag_name("style")
            && n.text().is_some_and(|s| s.contains("filter:url(#missing)")))
    );
}
