use std::collections::BTreeMap;
use std::path::Path;

use super::{
    resolve_codepoints, resolve_generate_webfonts_options, resolved_font_types,
    validate_font_type_order, validate_generate_webfonts_options,
};
use crate::input::LoadedSvgFile;
use crate::{
    FontType, FontVariant, FormatOptions, GenerateWebfontsOptions, MissingGlyphBehavior,
    MissingGlyphOptions, Woff2FormatOptions,
};

fn variant(name: &str, weight: Option<u16>, is_default: bool) -> FontVariant {
    FontVariant {
        name: name.to_owned(),
        files: vec![format!("{name}.svg")],
        weight,
        default: is_default.then_some(true),
    }
}

fn variant_options() -> GenerateWebfontsOptions {
    GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec![],
        types: Some(vec![FontType::Woff2]),
        variants: Some(vec![
            variant("small", Some(300), false),
            variant("large", Some(700), true),
        ]),
        ..Default::default()
    }
}

fn validation_error(options: GenerateWebfontsOptions, expected: &str) {
    let error = validate_generate_webfonts_options(&options).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        error.to_string().contains(expected),
        "expected {error:?} to contain {expected:?}",
    );
}

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
    let message = error.to_string();
    assert!(message.contains("Invalid font type order: 'woff' is not present in 'types'."));
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

#[test]
fn accepts_non_empty_files_without_variants() {
    validate_generate_webfonts_options(&GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        types: Some(vec![FontType::Woff2]),
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn accepts_empty_top_level_files_with_valid_variants() {
    validate_generate_webfonts_options(&variant_options()).unwrap();
}

#[test]
fn rejects_top_level_files_with_variants() {
    let mut options = variant_options();
    options.files.push("icon.svg".to_owned());
    validation_error(options, "options.files");
}

#[test]
fn rejects_fewer_than_two_variants() {
    let mut options = variant_options();
    options.variants.as_mut().unwrap().pop();
    validation_error(options, "options.variants");
}

#[test]
fn rejects_variant_with_empty_files() {
    let mut options = variant_options();
    options.variants.as_mut().unwrap()[0].files.clear();
    validation_error(options, "options.variants[0].files");
}

#[test]
fn rejects_svg_output_with_variants() {
    let mut options = variant_options();
    options.types = Some(vec![FontType::Svg]);
    validation_error(options, "options.types");
}

#[test]
fn rejects_incremental_mode_with_variants() {
    let mut options = variant_options();
    options.incremental = Some(true);
    validation_error(options, "options.incremental");
}

#[test]
fn rejects_invalid_variant_names() {
    for (name, expected) in [
        ("", "options.variants[0].name"),
        ("small icon", "whitespace"),
        ("small\u{2003}icon", "whitespace"),
        ("small\0icon", "NUL"),
    ] {
        let mut options = variant_options();
        options.variants.as_mut().unwrap()[0].name = name.to_owned();
        validation_error(options, expected);
    }
}

#[test]
fn rejects_duplicate_variant_names() {
    let mut options = variant_options();
    options.variants.as_mut().unwrap()[1].name = "small".to_owned();
    validation_error(options, "options.variants[1].name");
}

#[test]
fn requires_exactly_one_default_variant() {
    let mut no_default = variant_options();
    no_default.variants.as_mut().unwrap()[1].default = None;
    validation_error(no_default, "default");

    let mut multiple_defaults = variant_options();
    multiple_defaults.variants.as_mut().unwrap()[0].default = Some(true);
    validation_error(multiple_defaults, "default");

    validate_generate_webfonts_options(&variant_options()).unwrap();
}

#[test]
fn validates_explicit_weights() {
    for (weight, expected) in [
        (0, "options.variants[0].weight"),
        (1001, "options.variants[0].weight"),
    ] {
        let mut options = variant_options();
        options.variants.as_mut().unwrap()[0].weight = Some(weight);
        validation_error(options, expected);
    }

    let mut duplicate = variant_options();
    duplicate.variants.as_mut().unwrap()[1].weight = Some(300);
    validation_error(duplicate, "weight");

    let mut descending = variant_options();
    descending.variants.as_mut().unwrap()[0].weight = Some(700);
    descending.variants.as_mut().unwrap()[1].weight = Some(300);
    validation_error(descending, "weight");

    validate_generate_webfonts_options(&variant_options()).unwrap();
}

#[test]
fn accepts_mixed_automatic_and_explicit_weights_without_resolving_them() {
    let mut options = variant_options();
    options.variants.as_mut().unwrap()[0].weight = None;
    validate_generate_webfonts_options(&options).unwrap();
}

#[test]
fn rejects_top_level_font_weight_with_variants() {
    let mut options = variant_options();
    options.font_weight = Some("400".to_owned());
    validation_error(options, "options.fontWeight");
}

#[test]
fn validates_missing_glyph_policies() {
    validate_generate_webfonts_options(&variant_options()).unwrap();

    for behavior in [MissingGlyphBehavior::Blank, MissingGlyphBehavior::Error] {
        let mut options = variant_options();
        options.missing_glyphs = Some(MissingGlyphOptions {
            behavior,
            variant: None,
        });
        validate_generate_webfonts_options(&options).unwrap();
    }

    let mut fallback = variant_options();
    fallback.missing_glyphs = Some(MissingGlyphOptions {
        behavior: MissingGlyphBehavior::Fallback,
        variant: Some("small".to_owned()),
    });
    validate_generate_webfonts_options(&fallback).unwrap();
}

#[test]
fn rejects_invalid_missing_glyph_fallbacks() {
    for variant in [None, Some("unknown".to_owned())] {
        let mut options = variant_options();
        options.missing_glyphs = Some(MissingGlyphOptions {
            behavior: MissingGlyphBehavior::Fallback,
            variant,
        });
        validation_error(options, "options.missingGlyphs.variant");
    }

    for behavior in [MissingGlyphBehavior::Blank, MissingGlyphBehavior::Error] {
        let mut options = variant_options();
        options.missing_glyphs = Some(MissingGlyphOptions {
            behavior,
            variant: Some("small".to_owned()),
        });
        validation_error(options, "options.missingGlyphs.variant");
    }
}

#[test]
fn rejects_missing_glyph_policy_without_variants() {
    let mut options = GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        ..Default::default()
    };
    options.missing_glyphs = Some(MissingGlyphOptions {
        behavior: MissingGlyphBehavior::Blank,
        variant: None,
    });
    validation_error(options, "options.missingGlyphs");
}

#[test]
fn validates_variant_class_prefix() {
    validate_generate_webfonts_options(&variant_options()).unwrap();

    for prefix in ["", "icon prefix", "icon\u{2003}", "icon\0"] {
        let mut options = variant_options();
        options.variant_class_prefix = Some(prefix.to_owned());
        validation_error(options, "options.variantClassPrefix");
    }
}

#[test]
fn rejects_variant_class_prefix_without_variants() {
    let options = GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["icon.svg".to_owned()],
        variant_class_prefix: Some("weight--".to_owned()),
        ..Default::default()
    };
    validation_error(options, "options.variantClassPrefix");
}

#[test]
fn rejects_template_variant_class_prefix_with_variants() {
    let mut options = variant_options();
    options.template_options = Some(serde_json::Map::from_iter([(
        "variantClassPrefix".to_owned(),
        serde_json::Value::String("weight--".to_owned()),
    )]));
    validation_error(options, "options.templateOptions.variantClassPrefix");
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
    let message = error.to_string();
    assert!(message.contains("\"options.cssTemplate\" must not be empty."));
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
    let message = error.to_string();
    assert!(message.contains("\"options.htmlTemplate\" must not be empty."));
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

#[test]
fn rejects_exhausted_codepoint_space() {
    let source_files = vec![loaded_svg_file("first.svg"), loaded_svg_file("second.svg")];
    let error = resolve_codepoints(&source_files, &BTreeMap::new(), u32::MAX)
        .expect_err("a second glyph cannot follow u32::MAX");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("codepoint"));
}

#[test]
fn propagates_codepoint_exhaustion_while_finalizing() {
    let mut options = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["first.svg".to_owned(), "second.svg".to_owned()],
        start_codepoint: Some(u32::MAX),
        ..Default::default()
    })
    .unwrap();
    let source_files = vec![loaded_svg_file("first.svg"), loaded_svg_file("second.svg")];

    let error = super::finalize_generate_webfonts_options(&mut options, &source_files)
        .expect_err("finalization must propagate exhausted codepoints");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}
