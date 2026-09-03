use std::collections::BTreeMap;
use std::path::Path;

use super::{
    encode_filename_component, resolve_codepoints, resolve_generate_webfonts_options,
    resolve_variant_weights, resolved_font_types, serialize_css_identifier,
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
fn serializes_css_identifiers() {
    for (token, expected) in [
        ("icon--small", "icon--small"),
        ("icon--a.b", "icon--a\\.b"),
        ("icon--a#b", "icon--a\\#b"),
        ("icon--a:b", "icon--a\\:b"),
        ("icon--a/b", "icon--a\\/b"),
        ("icon--a\\b", "icon--a\\\\b"),
        ("icon--größe", "icon--größe"),
        ("1small", "\\31 small"),
        ("-1small", "-\\31 small"),
        ("\0", "�"),
        ("-", "\\-"),
    ] {
        assert_eq!(serialize_css_identifier(token), expected);
    }
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

fn resolved_weights(spec: &[(Option<u16>, bool)]) -> std::io::Result<Vec<u16>> {
    let variants = spec
        .iter()
        .enumerate()
        .map(|(index, (weight, is_default))| {
            variant(&format!("variant-{index}"), *weight, *is_default)
        })
        .collect::<Vec<_>>();
    let default_index = spec.iter().position(|(_, is_default)| *is_default).unwrap();
    resolve_variant_weights(&variants, default_index)
}

#[test]
fn resolves_automatic_weights_from_the_default() {
    for (spec, expected) in [
        (vec![(None, true), (None, false)], vec![400, 500]),
        (vec![(None, false), (None, true)], vec![300, 400]),
        (
            vec![(None, false), (None, true), (None, false)],
            vec![300, 400, 500],
        ),
        (
            vec![(Some(500), true), (None, false), (None, false)],
            vec![500, 600, 700],
        ),
    ] {
        assert_eq!(resolved_weights(&spec).unwrap(), expected);
    }
}

#[test]
fn resolves_mixed_weights_and_crowded_intervals() {
    assert_eq!(
        resolved_weights(&[
            (Some(100), false),
            (None, false),
            (None, true),
            (None, false),
            (Some(700), false),
        ])
        .unwrap(),
        [100, 300, 400, 500, 700],
    );
    assert_eq!(
        resolved_weights(&[
            (None, true),
            (None, false),
            (None, false),
            (Some(550), false),
        ])
        .unwrap(),
        [400, 450, 500, 550],
    );
}

#[test]
fn resolves_the_full_css_weight_range() {
    let mut canonical = vec![(None, false); 10];
    canonical[3].1 = true;
    assert_eq!(
        resolved_weights(&canonical).unwrap(),
        (100..=1000).step_by(100).collect::<Vec<_>>()
    );

    let mut spec = vec![(None, false); 1000];
    spec[399].1 = true;

    assert_eq!(
        resolved_weights(&spec).unwrap(),
        (1..=1000).collect::<Vec<_>>()
    );
}

#[test]
fn resolves_weights_deterministically() {
    let spec = [
        (Some(1), false),
        (None, false),
        (None, true),
        (None, false),
        (Some(1000), false),
    ];

    assert_eq!(
        resolved_weights(&spec).unwrap(),
        resolved_weights(&spec).unwrap()
    );
}

#[test]
fn rejects_conflicting_or_exhausted_weight_intervals() {
    for spec in [
        vec![(None, true), (Some(400), false)],
        vec![(Some(500), false), (None, false), (None, true)],
    ] {
        let error = resolved_weights(&spec).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("weight"));
    }

    let mut exhausted = vec![(None, false); 1001];
    exhausted[0] = (Some(1), true);
    let error = resolved_weights(&exhausted).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("cannot fit"));
}

#[test]
fn encodes_filesystem_safe_variant_names() {
    for (name, expected) in [
        ("small", "small"),
        ("a/b", "a~2Fb"),
        ("a\\b", "a~5Cb"),
        ("..", "~2E~2E"),
        ("café", "caf~C3~A9"),
        ("CON", "~43ON"),
        ("COM1", "~43OM1"),
        ("LPT9", "~4CPT9"),
        ("\u{1}", "~01"),
        ("a b", "a~20b"),
    ] {
        assert_eq!(encode_filename_component(name), expected);
    }
}

#[test]
fn resolver_rejects_variants_without_a_default() {
    let mut options = variant_options();
    for variant in options.variants.as_mut().unwrap() {
        variant.default = None;
    }

    let error = resolve_generate_webfonts_options(options)
        .err()
        .expect("expected unresolved variants without a default to fail");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("must contain a default variant"));
}

#[test]
fn resolves_ordered_variant_metadata() {
    let mut options = variant_options();
    options.variant_class_prefix = Some("weight--".to_owned());
    options.variants = Some(vec![
        variant("small", None, false),
        variant("large/alt", None, true),
    ]);

    let resolved = resolve_generate_webfonts_options(options).unwrap();
    let variants = resolved.variants.unwrap();

    assert_eq!(variants.default_index, 1);
    assert_eq!(variants.variants[0].weight, 300);
    assert_eq!(variants.variants[0].name, "small");
    assert_eq!(variants.variants[0].files, ["small.svg"]);
    assert_eq!(variants.variants[0].class_name, "weight--small");
    assert_eq!(variants.variants[0].selector, "weight--small");
    assert_eq!(variants.variants[0].filename_component, "small");
    assert_eq!(variants.variants[1].filename_component, "large~2Falt");
}

#[test]
fn rejects_case_insensitive_filename_collisions() {
    let mut options = variant_options();
    options.variants = Some(vec![
        variant("small", None, true),
        variant("Small", None, false),
    ]);

    let error = resolve_generate_webfonts_options(options)
        .err()
        .expect("expected filename collision to fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("filename"));
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
fn defaults_missing_glyph_policy_to_blank() {
    let resolved = resolve_generate_webfonts_options(variant_options()).unwrap();

    assert!(resolved.missing_glyphs.behavior == MissingGlyphBehavior::Blank);
    assert!(resolved.missing_glyphs.variant.is_none());
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

    let resolved = resolve_codepoints(
        source_files.iter().map(|file| file.glyph_name.as_str()),
        &BTreeMap::new(),
        0xF101,
    )
    .unwrap();

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

    let resolved = resolve_codepoints(
        source_files.iter().map(|file| file.glyph_name.as_str()),
        &explicit,
        0xF101,
    )
    .unwrap();

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

    let resolved = resolve_codepoints(
        source_files.iter().map(|file| file.glyph_name.as_str()),
        &BTreeMap::new(),
        0xF101,
    )
    .unwrap();

    assert_eq!(resolved.get(""), Some(&0xF101));
}

#[test]
fn rejects_exhausted_codepoint_space() {
    let source_files = vec![loaded_svg_file("first.svg"), loaded_svg_file("second.svg")];
    let error = resolve_codepoints(
        source_files.iter().map(|file| file.glyph_name.as_str()),
        &BTreeMap::new(),
        u32::MAX,
    )
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
