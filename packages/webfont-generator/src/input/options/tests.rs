use std::collections::BTreeMap;
use std::path::Path;

use super::{
    resolve_codepoints, resolve_generate_webfonts_options, resolved_font_types,
    validate_font_type_order, validate_generate_webfonts_options,
};
use crate::input::LoadedSvgFile;
use crate::{FontType, FormatOptions, GenerateWebfontsOptions, Woff2FormatOptions};

fn loaded_svg_file(path: &str) -> LoadedSvgFile {
    LoadedSvgFile {
        contents: "<svg />".into(),
        glyph_name: Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned(),
        path: path.to_owned(),
    }
}

#[test]
fn rejects_order_entries_that_are_not_present_in_types() {
    let options = GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec![],
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg, FontType::Woff]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };

    let error = validate_font_type_order(&options, &resolved_font_types(&options)).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("Invalid font type order: 'woff' is not present in 'types'.")
    );
}

#[test]
fn rejects_an_empty_dest() {
    let options = GenerateWebfontsOptions {
        dest: String::new(),
        files: vec!["icon.svg".to_owned()],
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };

    let error = validate_generate_webfonts_options(&options).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("\"options.dest\" is empty."));
}

#[test]
fn rejects_empty_files() {
    let options = GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec![],
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };

    let error = validate_generate_webfonts_options(&options).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("\"options.files\" is empty."));
}

fn options_with_woff2_quality(quality: u8) -> GenerateWebfontsOptions {
    GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        format_options: Some(FormatOptions {
            woff2: Some(Woff2FormatOptions {
                compression_quality: Some(quality),
            }),
            ..Default::default()
        }),
        html: Some(false),
        types: Some(vec![FontType::Woff2]),
        ..Default::default()
    }
}

#[test]
fn rejects_woff2_compression_quality_above_11() {
    let error = validate_generate_webfonts_options(&options_with_woff2_quality(12)).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains(
        "\"options.formatOptions.woff2.compressionQuality\" must be between 0 and 11, got 12."
    ));
}

#[test]
fn accepts_woff2_compression_quality_of_11() {
    validate_generate_webfonts_options(&options_with_woff2_quality(11))
        .expect("compression quality 11 is the upper bound and must be accepted");
}

#[test]
fn rejects_empty_css_template() {
    let error = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css_template: Some(String::new()),
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        ..Default::default()
    })
    .err()
    .expect("expected empty css template to fail");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("\"options.cssTemplate\" must not be empty.")
    );
}

#[test]
fn rejects_empty_html_template() {
    let error = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        html_template: Some(String::new()),
        ..Default::default()
    })
    .err()
    .expect("expected empty html template to fail");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("\"options.htmlTemplate\" must not be empty.")
    );
}

#[test]
fn resolves_write_defaults_from_dest_and_font_name() {
    let resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        font_name: Some("iconfont".to_owned()),
        ..Default::default()
    })
    .unwrap();

    assert!(resolved.write_files);
    assert_eq!(resolved.css_dest, "artifacts/iconfont.css");
    assert_eq!(resolved.html_dest, "artifacts/iconfont.html");
}

#[test]
fn rejects_nonexistent_css_template_when_css_is_true() {
    let error = validate_generate_webfonts_options(&GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        ..Default::default()
    })
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("cssTemplate"));
}

#[test]
fn allows_nonexistent_css_template_when_css_is_false() {
    validate_generate_webfonts_options(&GenerateWebfontsOptions {
        css: Some(false),
        css_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn rejects_nonexistent_html_template_when_html_is_true() {
    let error = validate_generate_webfonts_options(&GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        html: Some(true),
        html_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
        ..Default::default()
    })
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("htmlTemplate"));
}

#[test]
fn allows_nonexistent_html_template_when_html_is_false() {
    validate_generate_webfonts_options(&GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        html: Some(false),
        html_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn resolves_missing_codepoints_in_source_file_order() {
    let source_files = vec![
        loaded_svg_file("/tmp/icons/arrow-left.svg"),
        loaded_svg_file("/tmp/icons/arrow-right.svg"),
    ];

    let resolved = resolve_codepoints(&source_files, &BTreeMap::new(), 0xF101).unwrap();

    assert_eq!(resolved.get("arrow-left"), Some(&0xF101));
    assert_eq!(resolved.get("arrow-right"), Some(&0xF102));
}

#[test]
fn preserves_explicit_codepoints_and_skips_used_values() {
    let source_files = vec![
        loaded_svg_file("/tmp/icons/arrow-left.svg"),
        loaded_svg_file("/tmp/icons/arrow-right.svg"),
        loaded_svg_file("/tmp/icons/check.svg"),
    ];
    let explicit = BTreeMap::from([
        ("arrow-left".to_owned(), 0xF105),
        ("check".to_owned(), 0xF101),
    ]);

    let resolved = resolve_codepoints(&source_files, &explicit, 0xF101).unwrap();

    assert_eq!(resolved.get("arrow-left"), Some(&0xF105));
    assert_eq!(resolved.get("check"), Some(&0xF101));
    assert_eq!(resolved.get("arrow-right"), Some(&0xF102));
}

#[test]
fn assigns_a_codepoint_to_an_empty_glyph_name() {
    let source_files = vec![LoadedSvgFile {
        contents: "<svg />".into(),
        glyph_name: String::new(),
        path: "/tmp/icons/..".to_owned(),
    }];

    let resolved = resolve_codepoints(&source_files, &BTreeMap::new(), 0xF101).unwrap();

    assert_eq!(resolved.get(""), Some(&0xF101));
}
