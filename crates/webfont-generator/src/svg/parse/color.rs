//! Root markers distinguish ordinary inherited initial paint from fixed paint.
//! Advanced attribute-dependent selectors and CSS resets are best-effort.
use std::collections::HashSet;
use std::io::Error;

pub(super) fn marked_tree(
    source: &str,
    document: &roxmltree::Document<'_>,
    original: usvg::Tree,
    options: &usvg::Options,
) -> Result<(usvg::Tree, Option<usvg::Color>), Error> {
    let root = document.root_element();
    let inherited_color = root.attribute_node("color").filter(|attribute| {
        attribute
            .value()
            .trim()
            .eq_ignore_ascii_case("currentColor")
    });
    if root.attribute("fill").is_some()
        && root.attribute("color").is_some()
        && inherited_color.is_none()
    {
        // No defaults need insertion. Keep the resolved tree, with no marker.
        return Ok((original, None));
    }
    let mut used = HashSet::new();
    collect_resolved_colors(original.root(), &mut used);
    let value = (0..=0xffffff)
        .find(|value| !used.contains(value))
        .ok_or_else(|| Error::other("No unused SVG foreground marker color remains."))?;
    let marker = usvg::Color::new_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8);
    // Use the parsed root's range, so declarations, comments, namespaces and
    // quoted '>' characters cannot redirect insertion into the wrong element.
    let start = root.range().start + 1;
    let end = start
        + source[start..]
            .find(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>'))
            .ok_or_else(|| Error::other("Missing SVG root name terminator."))?;
    let mut attributes = String::new();
    for name in ["fill", "color"] {
        if root.attribute(name).is_none() {
            attributes.push_str(&format!(r##" {name}="#{value:06x}""##));
        }
    }
    let mut rewritten = source.to_owned();
    if let Some(attribute) = inherited_color {
        // On the root, color=currentColor inherits the host color. Replace its
        // value at presentation-attribute priority, so authored CSS still wins.
        rewritten.replace_range(attribute.range_value(), &format!("#{value:06x}"));
    }
    rewritten.insert_str(end, &attributes);
    let document = super::parse_svg_document(&rewritten)?;
    let tree = usvg::Tree::from_xmltree(&document, options)
        .map_err(|error| Error::other(format!("Failed to extract SVG paint: {error}")))?;
    Ok((tree, Some(marker)))
}

fn collect_resolved_colors(group: &usvg::Group, used: &mut HashSet<u32>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(group) => collect_resolved_colors(group, used),
            usvg::Node::Path(path) => {
                if let Some(fill) = path.fill()
                    && let usvg::Paint::Color(color) = fill.paint()
                {
                    used.insert(
                        (u32::from(color.red) << 16)
                            | (u32::from(color.green) << 8)
                            | u32::from(color.blue),
                    );
                }
            }
            _ => {}
        }
    }
}
