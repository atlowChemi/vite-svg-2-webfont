use std::sync::Arc;

use super::*;
use crate::input::LoadedSvgFile;

fn parse(svg: &str, preserve_aspect_ratio: bool) -> Result<ParsedGlyph, Error> {
    let source_file = LoadedSvgFile {
        contents: Arc::from(svg),
        glyph_name: "inline".to_owned(),
        path: "inline.svg".to_owned(),
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
fn parses_viewboxes_and_dimension_prefixes() {
    assert_eq!(parse_view_box("1, 2, 30, 40"), Some((1.0, 2.0, 30.0, 40.0)));
    assert_eq!(parse_view_box("0 0 10"), None);
    assert_eq!(parse_view_box("0 0 ten 10"), None);

    assert_eq!(parse_number_prefix("250px"), Some(250.0));
    assert_eq!(parse_number_prefix(" -1.5e2pt"), Some(-150.0));
    assert_eq!(parse_number_prefix("none"), None);
    assert_eq!(parse_number_prefix("."), None);
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
fn builds_only_the_required_root_viewbox_correction() {
    let metrics = RootSvgMetrics {
        current_preserve_aspect_ratio: true,
        view_box_height: 100.0,
        view_box_width: 100.0,
        view_box_x: 0.0,
        view_box_y: 0.0,
        viewport_height: 100.0,
        viewport_width: 200.0,
    };

    assert!(
        build_root_viewbox_correction(&metrics, true)
            .unwrap()
            .is_none()
    );
    let correction = build_root_viewbox_correction(&metrics, false)
        .unwrap()
        .expect("stretching should require a correction");
    assert!(transforms_close(
        correction,
        Transform::from_row(2.0, 0.0, 0.0, 1.0, -100.0, 0.0)
    ));
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
