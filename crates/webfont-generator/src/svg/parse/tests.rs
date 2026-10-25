use std::sync::Arc;

use super::*;
use crate::input::LoadedSvgSource;

fn parse(svg: &str, preserve_aspect_ratio: bool) -> Result<ParsedGlyph, Error> {
    let source_file = LoadedSvgSource {
        contents: Arc::from(svg),
        glyph_name: "inline".to_owned(),
        id: "inline.svg".to_owned(),
    };
    parse_svg_glyph(
        &GlyphWorkItem {
            codepoint: 0xe001,
            index: 0,
            name: "inline",
            source_file: &source_file,
        },
        preserve_aspect_ratio,
        &usvg::Options::default(),
        false,
    )
}

fn parse_error(svg: &str) -> Error {
    match parse(svg, false) {
        Err(error) => error,
        Ok(_) => panic!("expected SVG parsing to fail"),
    }
}

#[test]
fn reports_xml_viewbox_and_usvg_errors_distinctly() {
    let error = match parse_svg_document("<svg><") {
        Err(error) => error,
        Ok(_) => panic!("expected XML inspection to fail"),
    };
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("Failed to inspect SVG root element")
    );

    let error = parse_error(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 nope 10"/>"#);
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("Failed to parse the SVG viewBox")
    );

    let error = parse_error("<html/>");
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("Failed to parse SVG fixture 'inline.svg'")
    );
}

#[test]
fn parses_viewboxes() {
    assert_eq!(parse_view_box("1, 2, 30, 40"), Some((1.0, 2.0, 30.0, 40.0)));
    assert_eq!(parse_view_box("0 0 10"), None);
    assert_eq!(parse_view_box("0 0 ten 10"), None);
}

#[test]
fn parses_filled_paths_without_a_viewbox() {
    let glyph = parse(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10">
            <path d="M0 0H20V10H0Z"/>
        </svg>"#,
        false,
    )
    .unwrap();

    assert_eq!(glyph.codepoint, 0xe001);
    assert_eq!(glyph.name, "inline");
    assert_eq!((glyph.width, glyph.height), (20.0, 10.0));
    assert_eq!(glyph.paths.len(), 1);
}

#[test]
fn root_transform_wraps_the_requested_viewbox_mapping() {
    for (preserve, expected) in [
        (false, (20.0, 30.0, 80.0, 50.0)),
        (true, (65.0, 30.0, 95.0, 50.0)),
    ] {
        for aspect in ["", r#" preserveAspectRatio="none""#] {
            let glyph = parse(
                &format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 100 100" transform="translate(10 20)"{aspect}>
                    <path d="M5 10H35V30H5Z"/>
                </svg>"#),
                preserve,
            ).unwrap();
            let bounds = glyph.paths[0].bounds();
            assert_eq!(
                (bounds.left(), bounds.top(), bounds.right(), bounds.bottom()),
                expected,
                "preserve={preserve}, aspect={aspect}"
            );
        }
    }
}

#[test]
fn root_aspect_ratio_keeps_authored_alignment_and_nested_transforms() {
    let svg = r#"<?xml version="1.0"?><!-- <svg> -->
        <s:svg xmlns:s="http://www.w3.org/2000/svg" data-note=">" width="200" height="100"
            viewBox="0 0 100 100" preserveAspectRatio="xMinYMin meet" transform="translate(10 20)">
            <s:g transform="translate(2 3)"><s:path d="M5 10H35V30H5Z"/></s:g>
        </s:svg>"#;
    for (preserve, expected) in [
        (true, (17.0, 33.0, 47.0, 53.0)),
        (false, (24.0, 33.0, 84.0, 53.0)),
    ] {
        let glyph = parse(svg, preserve).unwrap();
        let bounds = glyph.paths[0].bounds();
        assert_eq!(
            (bounds.left(), bounds.top(), bounds.right(), bounds.bottom()),
            expected
        );
    }
}

#[test]
fn viewbox_only_icons_skip_rewriting_but_dimension_attributes_do_not() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="5 10 100 50" transform="translate(10 20)"><path d="M5 10H35V30H5Z"/></svg>"#;
    for source in [
        svg.to_owned(),
        svg.replace("viewBox=", r#"style="width:200px;height:50px" viewBox="#),
        svg.replace(
            "<path",
            "<style>svg { width:200px; height:50px }</style><path",
        ),
    ] {
        let document = parse_svg_document(&source).unwrap();
        assert!(matches!(
            root_aspect_ratio(&source, &document, false).unwrap(),
            Cow::Borrowed(_)
        ));
        let stretched = parse(&source, false).unwrap();
        let preserved = parse(&source, true).unwrap();
        assert_eq!(stretched.paths[0], preserved.paths[0]);
    }
    for attributes in [
        r#"width="200" height="50""#,
        r#"xmlns:s="http://www.w3.org/2000/svg" s:width="200" s:height="50""#,
    ] {
        let source = svg.replace("viewBox=", &format!("{attributes} viewBox="));
        let document = parse_svg_document(&source).unwrap();
        assert!(matches!(
            root_aspect_ratio(&source, &document, false).unwrap(),
            Cow::Owned(_)
        ));
        assert_eq!(
            parse(&source, false).unwrap().paths[0].bounds().width(),
            60.0
        );
    }
}

#[test]
fn collects_stroke_caps_joins_and_dashes_but_ignores_images() {
    let glyph = parse(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">
            <path d="M10 10H90" fill="none" stroke="black" stroke-width="4" stroke-linecap="butt"/>
            <path d="M10 20H90" fill="none" stroke="black" stroke-width="4" stroke-linecap="round"/>
            <path d="M10 30H90" fill="none" stroke="black" stroke-width="4" stroke-linecap="square"/>
            <path d="M10 45L50 35L90 45" fill="none" stroke="black" stroke-width="4" stroke-linejoin="miter"/>
            <path d="M10 55L50 45L90 55" fill="none" stroke="black" stroke-width="4" stroke-linejoin="miter-clip"/>
            <path d="M10 65L50 55L90 65" fill="none" stroke="black" stroke-width="4" stroke-linejoin="round"/>
            <path d="M10 75L50 65L90 75" fill="none" stroke="black" stroke-width="4" stroke-linejoin="bevel"/>
            <path d="M10 90H90" fill="none" stroke="black" stroke-width="4" stroke-dasharray="8 4" stroke-dashoffset="2"/>
            <image width="1" height="1" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="/>
        </svg>"#,
        false,
    )
    .unwrap();

    assert_eq!(glyph.paths.len(), 8);
    assert!(glyph.paths.iter().all(|path| path.bounds().width() > 0.0));
}
