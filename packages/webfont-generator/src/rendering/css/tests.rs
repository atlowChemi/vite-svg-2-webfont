use super::{
    SharedTemplateData, build_css_context, calc_hash, make_ctx, make_src, make_urls,
    render_css_with_context, template_dependencies,
};
use crate::input::LoadedSvgFile;
use crate::types::ResolvedGenerateWebfontsOptions;
use crate::{
    FontType, FormatOptions, GenerateWebfontsOptions, SvgFormatOptions, TtfFormatOptions,
    WoffFormatOptions,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};

fn render_css(
    options: &ResolvedGenerateWebfontsOptions,
    source_files: &[LoadedSvgFile],
) -> Result<String, Error> {
    let shared = SharedTemplateData::new(options, source_files)?;
    let ctx = build_css_context(options, &shared);
    render_css_with_context(&shared, &ctx)
}

use crate::test_helpers::{fixture_source_files, resolve_options, write_temp_template};

#[test]
fn hash_matches_expected_value_for_known_options() {
    let options = GenerateWebfontsOptions {
        ascent: Some(1000.0),
        center_horizontally: Some(true),
        center_vertically: Some(false),
        css: Some(false),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        descent: Some(120.0),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        fixed_width: Some(false),
        format_options: Some(FormatOptions {
            svg: Some(SvgFormatOptions {
                font_id: Some("iconfont".to_owned()),
                metadata: Some("svg-meta".to_owned()),
                ..Default::default()
            }),
            ttf: Some(TtfFormatOptions {
                copyright: Some("copyright".to_owned()),
                description: Some("description".to_owned()),
                ts: Some(1_484_141_760_000),
                url: Some("https://example.com".to_owned()),
                version: Some("Version 1.0".to_owned()),
            }),
            woff: Some(WoffFormatOptions {
                metadata: Some("woff-meta".to_owned()),
            }),
            woff2: None,
        }),
        html: Some(false),
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_owned()),
        font_style: Some("normal".to_owned()),
        font_weight: Some("400".to_owned()),
        ligature: Some(false),
        normalize: Some(true),
        order: Some(vec![FontType::Woff2, FontType::Svg, FontType::Ttf]),
        optimize_output: Some(false),
        preserve_aspect_ratio: Some(false),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        types: Some(vec![FontType::Svg, FontType::Ttf, FontType::Woff2]),
        ..Default::default()
    };

    let options = resolve_options(options);
    let source_files = vec![LoadedSvgFile {
        contents: fs::read_to_string(&options.files[0])
            .expect("fixture should load")
            .into(),
        glyph_name: "add".to_owned(),
        path: options.files[0].clone(),
    }];
    let hash1 = calc_hash(&options, &source_files);
    let hash2 = calc_hash(&options, &source_files);

    assert_eq!(hash1, hash2, "hash should be deterministic across calls");
    assert_eq!(hash1.len(), 32, "hash should be a 32-char hex string");
}

#[test]
fn make_urls_uses_hash_and_requested_type_order() {
    let options = GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        types: Some(vec![FontType::Svg, FontType::Woff2]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = vec![LoadedSvgFile {
        contents: fs::read_to_string(&options.files[0])
            .expect("fixture should load")
            .into(),
        glyph_name: "add".to_owned(),
        path: options.files[0].clone(),
    }];
    let hash = calc_hash(&options, &source_files);

    let urls = make_urls(
        &options,
        &calc_hash(&options, &source_files),
        options.css_fonts_url.as_deref(),
    );

    assert_eq!(
        urls.get(&FontType::Svg),
        Some(&format!("iconfont.svg?{hash}"))
    );
    assert_eq!(
        urls.get(&FontType::Woff2),
        Some(&format!("iconfont.woff2?{hash}"))
    );
}

#[test]
fn make_urls_joins_against_css_fonts_url_and_normalizes_backslashes() {
    let options = GenerateWebfontsOptions {
        css: Some(false),
        css_fonts_url: Some("fonts\\nested\\".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        types: Some(vec![FontType::Ttf]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = vec![LoadedSvgFile {
        contents: fs::read_to_string(&options.files[0])
            .expect("fixture should load")
            .into(),
        glyph_name: "add".to_owned(),
        path: options.files[0].clone(),
    }];
    let hash = calc_hash(&options, &source_files);

    let urls = make_urls(
        &options,
        &calc_hash(&options, &source_files),
        options.css_fonts_url.as_deref(),
    );

    assert_eq!(
        urls.get(&FontType::Ttf),
        Some(&format!("fonts/nested/iconfont.ttf?{hash}"))
    );
}

#[test]
fn make_urls_preserves_leading_slash_when_css_fonts_url_is_only_slashes() {
    let options = GenerateWebfontsOptions {
        css: Some(false),
        css_fonts_url: Some("///".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = vec![LoadedSvgFile {
        contents: fs::read_to_string(&options.files[0])
            .expect("fixture should load")
            .into(),
        glyph_name: "add".to_owned(),
        path: options.files[0].clone(),
    }];
    let hash = calc_hash(&options, &source_files);

    let urls = make_urls(
        &options,
        &calc_hash(&options, &source_files),
        options.css_fonts_url.as_deref(),
    );

    assert_eq!(
        urls.get(&FontType::Svg),
        Some(&format!("/iconfont.svg?{hash}"))
    );
}

#[test]
fn make_src_uses_order_and_format_specific_url_templates() {
    let options = GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Eot, FontType::Svg, FontType::Woff2]),
        types: Some(vec![FontType::Svg, FontType::Eot, FontType::Woff2]),
        ..Default::default()
    };
    let urls = HashMap::from([
        (FontType::Svg, "iconfont.svg?svg-hash".to_owned()),
        (FontType::Eot, "iconfont.eot?eot-hash".to_owned()),
        (FontType::Woff2, "iconfont.woff2?woff2-hash".to_owned()),
    ]);

    let options = resolve_options(options);
    let src = make_src(&options, &urls);

    assert_eq!(
        src,
        concat!(
            "url(\"iconfont.eot?eot-hash?#iefix\") format(\"embedded-opentype\")",
            ",\n",
            "url(\"iconfont.svg?svg-hash#iconfont\") format(\"svg\")",
            ",\n",
            "url(\"iconfont.woff2?woff2-hash\") format(\"woff2\")"
        )
    );
}

#[test]
fn make_src_uses_upstream_default_order_when_order_is_not_provided() {
    let options = GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        types: Some(vec![
            FontType::Svg,
            FontType::Woff,
            FontType::Eot,
            FontType::Ttf,
        ]),
        ..Default::default()
    };
    let urls = HashMap::from([
        (FontType::Svg, "iconfont.svg?svg-hash".to_owned()),
        (FontType::Eot, "iconfont.eot?eot-hash".to_owned()),
        (FontType::Woff, "iconfont.woff?woff-hash".to_owned()),
        (FontType::Ttf, "iconfont.ttf?ttf-hash".to_owned()),
    ]);

    let options = resolve_options(options);
    let src = make_src(&options, &urls);

    assert_eq!(
        src,
        concat!(
            "url(\"iconfont.eot?eot-hash?#iefix\") format(\"embedded-opentype\")",
            ",\n",
            "url(\"iconfont.woff?woff-hash\") format(\"woff\")",
            ",\n",
            "url(\"iconfont.ttf?ttf-hash\") format(\"truetype\")",
            ",\n",
            "url(\"iconfont.svg?svg-hash#iconfont\") format(\"svg\")"
        )
    );
}

#[test]
fn make_ctx_builds_codepoints_and_merges_template_options() {
    let options = GenerateWebfontsOptions {
        css: Some(false),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        dest: "artifacts".to_owned(),
        files: vec![],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Ttf]),
        template_options: Some(Map::from_iter([
            (
                "baseSelector".to_owned(),
                Value::String(".glyph".to_owned()),
            ),
            (
                "fontName".to_owned(),
                Value::String("overridden".to_owned()),
            ),
        ])),
        types: Some(vec![FontType::Ttf]),
        ..Default::default()
    };
    let urls = HashMap::from([(FontType::Ttf, "iconfont.ttf?hash".to_owned())]);

    let options = resolve_options(options);
    let shared = SharedTemplateData::new(&options, &[]).unwrap();
    let ctx = make_ctx(&options, &urls, &shared);

    assert_eq!(
        ctx.get("fontName"),
        Some(&Value::String("overridden".to_owned()))
    );
    assert_eq!(
        ctx.get("src"),
        Some(&Value::String(
            "url(\"iconfont.ttf?hash\") format(\"truetype\")".to_owned()
        ))
    );
    assert_eq!(
        ctx.get("baseSelector"),
        Some(&Value::String(".glyph".to_owned()))
    );
    assert_eq!(
        ctx.get("classPrefix"),
        Some(&Value::String("icon-".to_owned()))
    );
    assert_eq!(
        ctx.get("codepoints"),
        Some(&Value::Object(Map::from_iter([(
            "add".to_owned(),
            Value::String("e001".to_owned()),
        )])))
    );
}

#[test]
fn render_css_renders_the_template_with_generated_urls() {
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg, FontType::Ttf]),
        types: Some(vec![FontType::Svg, FontType::Ttf]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = vec![LoadedSvgFile {
        contents: fs::read_to_string(&options.files[0])
            .expect("fixture should load")
            .into(),
        glyph_name: "add".to_owned(),
        path: options.files[0].clone(),
    }];

    let css = render_css(&options, &source_files).expect("css should render");

    assert!(css.contains("@font-face"));
    assert!(css.contains("font-family: \"iconfont\";"));
    assert!(css.contains("url(\"/assets/fonts/iconfont.svg?"));
    assert!(css.contains("format(\"svg\")"));
    assert!(css.contains("format(\"truetype\")"));
    assert!(css.contains(".icon-add:before"));
    assert!(css.contains("\\e001"));
}

#[test]
fn render_css_supports_static_custom_templates() {
    let template_path = write_temp_template("native-css-static-template", "custom css");
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(template_path),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let css = render_css(&options, &source_files).expect("css should render");

    assert_eq!(css, "custom css");
}

#[test]
fn render_css_supports_custom_templates_using_all_available_context_values() {
    let template_path = write_temp_template(
        "native-css-full-context-template",
        "{{fontName}}|{{{src}}}|{{baseSelector}}|{{classPrefix}}|{{codepoints.add}}|{{option}}",
    );
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(template_path),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        template_options: Some(Map::from_iter([(
            "option".to_owned(),
            Value::String("TEST".to_owned()),
        )])),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let css = render_css(&options, &source_files).expect("css should render");

    assert!(css.starts_with("iconfont|url(\"/assets/fonts/iconfont.svg?"));
    assert!(css.contains("#iconfont\") format(\"svg\")|.icon|icon-|e001|TEST"));
}

#[test]
fn render_css_rejects_invalid_handlebars_templates() {
    let template_path = write_temp_template("native-css-invalid-template", "{{#if}}");
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(template_path),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let error =
        render_css(&options, &source_files).expect_err("invalid handlebars syntax should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn default_css_hot_path_matches_handlebars_output() {
    use handlebars::Handlebars;

    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
        codepoints: Some(HashMap::from([
            ("add".to_owned(), 0xE001u32),
            ("remove".to_owned(), 0xE002u32),
            ("search".to_owned(), 0xE003u32),
        ])),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        start_codepoint: Some(0xE001),
        template_options: Some(Map::from_iter([
            ("baseSelector".to_owned(), Value::String(".icon".to_owned())),
            ("classPrefix".to_owned(), Value::String("icon-".to_owned())),
        ])),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);
    let shared_with_template = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = super::build_css_context(&options, &shared_with_template);

    // Render via Handlebars (the template path is set)
    let handlebars_output = {
        let source = fs::read_to_string(options.css_template.as_ref().unwrap()).unwrap();
        let registry = Handlebars::new();
        registry.render_template(&source, &ctx).unwrap()
    };

    // Render via hot path (no template = default)
    let hot_path_output = super::render_default_css(&ctx);

    assert_eq!(
        hot_path_output, handlebars_output,
        "CSS hot path output must match Handlebars output"
    );
}

#[test]
fn render_css_with_hbs_context_matches_direct_render_for_default_template() {
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        codepoints: Some(HashMap::from([
            ("add".to_owned(), 0xE001u32),
            ("remove".to_owned(), 0xE002u32),
        ])),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_css_context(&options, &shared);
    let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

    let direct = render_css_with_context(&shared, &ctx).unwrap();
    let via_hbs = super::render_css_with_hbs_context(&shared, &hbs_ctx, &ctx).unwrap();

    assert_eq!(
        via_hbs, direct,
        "render_css_with_hbs_context must match render_css_with_context"
    );
}

#[test]
fn render_css_with_hbs_context_matches_direct_render_for_custom_template() {
    let template_path = write_temp_template(
        "native-css-hbs-ctx",
        "@font-face { src: {{{src}}}; } {{#each codepoints}}.{{@key}}:before { content: \"\\\\{{this}}\"; }{{/each}}",
    );
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(template_path),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_css_context(&options, &shared);
    let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

    let direct = render_css_with_context(&shared, &ctx).unwrap();
    let via_hbs = super::render_css_with_hbs_context(&shared, &hbs_ctx, &ctx).unwrap();

    assert_eq!(
        via_hbs, direct,
        "render_css_with_hbs_context with custom template must match render_css_with_context"
    );
}

#[test]
fn render_css_with_src_swap_matches_manual_context_rewrite() {
    let template_path = write_temp_template(
        "native-css-src-swap",
        "@font-face { src: {{{src}}}; } .icon { font-family: {{fontName}}; }",
    );
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(template_path),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_css_context(&options, &shared);
    let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

    // Manual approach: clone Map, replace src, render
    let new_src = "url(\"/custom/path.woff2\") format(\"woff2\")";
    let mut manual_ctx = ctx.clone();
    manual_ctx.insert("src".to_owned(), Value::String(new_src.to_owned()));
    let expected = render_css_with_context(&shared, &manual_ctx).unwrap();

    // Optimized approach: in-place mutate hbs Context src field
    let mut hbs_ctx = hbs_ctx;
    let actual = super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, new_src).unwrap();

    assert_eq!(
        actual, expected,
        "render_css_with_src_mutate must produce identical output to manual Map rewrite"
    );

    // Verify original src was restored
    let restored_src = hbs_ctx
        .data()
        .as_object()
        .unwrap()
        .get("src")
        .unwrap()
        .as_str()
        .unwrap();
    let original_src = ctx.get("src").unwrap().as_str().unwrap();
    assert_eq!(
        restored_src, original_src,
        "original src should be restored after render"
    );
}

#[test]
fn render_css_with_src_mutate_produces_correct_results_on_repeated_calls() {
    let template_path = write_temp_template(
        "native-css-src-mutate-repeat",
        "@font-face { src: {{{src}}}; }",
    );
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(template_path),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_css_context(&options, &shared);
    let mut hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

    let src_a = "url(\"/a.woff2\") format(\"woff2\")";
    let src_b = "url(\"/b.woff\") format(\"woff\")";

    let result_a = super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, src_a).unwrap();
    let result_b = super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, src_b).unwrap();
    let result_a_again =
        super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, src_a).unwrap();

    assert!(result_a.contains(src_a), "first call should use src_a");
    assert!(result_b.contains(src_b), "second call should use src_b");
    assert_eq!(
        result_a, result_a_again,
        "repeated call with same src should produce identical output"
    );
    assert_ne!(
        result_a, result_b,
        "different src values should produce different output"
    );
}

#[test]
fn css_registry_rejects_invalid_template_syntax_on_first_access() {
    let template_path = write_temp_template("native-css-invalid-compile", "{{#if}}");
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(template_path),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });

    // SharedTemplateData::new succeeds (file is read, compilation is deferred)
    let shared = SharedTemplateData::new(&options, &[])
        .expect("init should succeed — template source is read but not compiled");

    // First access to css_registry triggers compilation and fails
    match shared.css_registry() {
        Err(error) => {
            assert_eq!(error.kind(), ErrorKind::InvalidData);
            assert!(
                error.to_string().contains("Failed to compile CSS template"),
                "error should mention CSS template: {error}"
            );
        }
        Ok(_) => panic!("invalid handlebars syntax should fail on first css_registry() access"),
    }
}

#[test]
fn shared_template_data_reads_source_but_does_not_compile_invalid_css_template_eagerly() {
    let template_path = write_temp_template("native-css-invalid-lazy", "{{#if}}");
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(false),
        css_template: Some(template_path),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });

    // Init succeeds — source is read but not compiled
    let shared = SharedTemplateData::new(&options, &[]);
    assert!(
        shared.is_ok(),
        "init should succeed even with invalid template content"
    );
}

// --- template_dependencies ---

#[test]
fn template_dependencies_detect_each_codepoints_and_parent_paths() {
    let deps = template_dependencies(
        "{{#each codepoints}}.{{../classPrefix}}{{@key}} { content: x }{{/each}}",
    );

    assert!(deps.codepoints);
    assert!(!deps.names);
    assert!(!deps.src);
    assert!(!deps.styles);
    assert!(!deps.dynamic);
}

#[test]
fn template_dependencies_detect_html_names_styles_and_known_helper_arg() {
    let deps = template_dependencies(
        "{{{styles}}} {{#each names}}{{removePeriods ../baseSelector}} {{@index}}{{/each}}",
    );

    assert!(deps.names);
    assert!(deps.styles);
    assert!(!deps.codepoints);
    assert!(!deps.src);
    assert!(!deps.dynamic);
}

#[test]
fn template_dependencies_normalizes_root_qualified_paths() {
    let deps = template_dependencies("{{@root.styles}} {{this.names}} {{@root/codepoints.a}}");

    assert!(deps.names);
    assert!(deps.styles);
    assert!(deps.codepoints);
    assert!(!deps.src);
    assert!(!deps.dynamic);
}

#[test]
fn template_dependencies_normalizes_whitespace_control_markers() {
    let deps = template_dependencies(
        "{{~styles}} {{#each names~}}{{@index}}{{/each}} {{~@root/codepoints.a~}}",
    );

    assert!(deps.names);
    assert!(deps.styles);
    assert!(deps.codepoints);
    assert!(!deps.src);
    assert!(!deps.dynamic);
}

#[test]
fn template_dependencies_marks_lookup_dynamic() {
    let deps = template_dependencies("{{lookup codepoints \"a\"}}");

    assert!(deps.dynamic);
}

#[test]
fn template_dependencies_marks_lookup_subexpression_dynamic() {
    let deps = template_dependencies("{{#each (lookup this field)}}{{this}}{{/each}}");

    assert!(deps.dynamic);
}

#[test]
fn template_dependencies_marks_block_params_dynamic() {
    let deps = template_dependencies(
        "{{#with this as |ctx|}}{{#each ctx.names}}{{this}}{{/each}}{{/with}}",
    );

    assert!(deps.dynamic);
}

#[test]
fn template_dependencies_normalizes_current_context_paths() {
    let deps = template_dependencies("{{#each ./names}}{{@index}}{{/each}}");

    assert!(deps.names);
    assert!(!deps.dynamic);
}

#[test]
fn template_dependencies_normalizes_unescaped_ampersand_paths() {
    let deps = template_dependencies("{{&styles}}{{&src}}");

    assert!(deps.styles);
    assert!(deps.src);
    assert!(!deps.dynamic);
}

#[test]
fn template_dependencies_marks_whole_context_reads_dynamic() {
    assert!(template_dependencies("{{this}}").dynamic);
    assert!(template_dependencies("{{.}}").dynamic);
    assert!(template_dependencies("{{@root}}").dynamic);
    assert!(template_dependencies("{{#each this}}{{@key}}={{this}}{{/each}}").dynamic);
    assert!(template_dependencies("{{#each .}}{{@key}}={{this}}{{/each}}").dynamic);
}

#[test]
fn template_dependencies_marks_partials_dynamic() {
    assert!(template_dependencies("{{> iconPreview}}").dynamic);
    assert!(template_dependencies("{{>iconPreview}}").dynamic);
    assert!(template_dependencies("{{#> iconPreview}}{{/iconPreview}}").dynamic);
}
