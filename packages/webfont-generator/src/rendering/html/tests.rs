use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

use super::{build_html_context, build_html_registry, render_html_with_context};
use crate::input::LoadedSvgFile;
use crate::rendering::css::SharedTemplateData;
use crate::rendering::paths::{path_to_slashes, relative_path};
use crate::types::ResolvedGenerateWebfontsOptions;
use crate::{FontType, GenerateWebfontsOptions};

use crate::test_helpers::{fixture_source_files, resolve_options, write_temp_template};

fn render_html(
    options: &ResolvedGenerateWebfontsOptions,
    source_files: &[LoadedSvgFile],
) -> Result<String, std::io::Error> {
    let shared = SharedTemplateData::new(options, source_files)?;
    let ctx = build_html_context(options, &shared, source_files, None)?;
    let registry = build_html_registry(options)?;
    render_html_with_context(options, registry.as_ref(), &ctx)
}

#[test]
fn render_html_renders_the_template_with_generated_styles_and_names() {
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_template: Some(format!("{}/templates/html.hbs", env!("CARGO_MANIFEST_DIR"))),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let html = render_html(&options, &source_files).expect("html should render");

    assert!(html.contains("<title>iconfont</title>"));
    assert!(html.contains("<h1>iconfont</h1>"));
    assert!(html.contains("preview__icon"));
    assert!(html.contains(">add<"));
    assert!(html.contains("class=\"icon icon-add\""));
    assert!(html.contains("url(\"iconfont.svg?"));
}

#[test]
fn relative_path_resolves_from_html_dest_to_font_dest() {
    let relative = relative_path(
        Path::new("/artifacts/preview"),
        Path::new("/artifacts/fonts"),
    );

    assert_eq!(path_to_slashes(relative), "../fonts");
}

#[test]
fn render_html_uses_font_paths_relative_to_html_dest() {
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_dest: Some("/artifacts/styles/iconfont.css".to_owned()),
        css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/ignored".to_owned()),
        dest: "/artifacts/fonts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_dest: Some("/artifacts/preview/iconfont.html".to_owned()),
        html_template: Some(format!("{}/templates/html.hbs", env!("CARGO_MANIFEST_DIR"))),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let html = render_html(&options, &source_files).expect("html should render");

    assert!(html.contains("url(\"../fonts/iconfont.svg?"));
}

#[test]
fn render_html_supports_static_custom_templates() {
    let template_path = write_temp_template("native-html-static-template", "custom html");
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_template: Some(template_path),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let html = render_html(&options, &source_files).expect("html should render");

    assert_eq!(html, "custom html");
}

#[test]
fn render_html_supports_custom_templates_using_all_available_context_values() {
    let template_path = write_temp_template(
        "native-html-full-context-template",
        "{{fontName}}|{{{styles}}}|{{baseSelector}}|{{classPrefix}}|{{codepoints.add}}|{{#each names}}{{this}}{{/each}}|{{option}}",
    );
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_template: Some(template_path),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        template_options: Some(serde_json::Map::from_iter([(
            "option".to_owned(),
            serde_json::Value::String("TEST".to_owned()),
        )])),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let html = render_html(&options, &source_files).expect("html should render");

    assert!(html.starts_with("iconfont|@font-face"));
    assert!(html.contains("|.icon|icon-|57345|add|TEST"));
    assert!(html.contains("url(\"iconfont.svg?"));
}

#[test]
fn render_html_rejects_invalid_handlebars_templates() {
    let template_path = write_temp_template("native-html-invalid-template", "{{#if}}");
    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        css_fonts_url: Some("/assets/fonts".to_owned()),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_template: Some(template_path),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);

    let error =
        render_html(&options, &source_files).expect_err("invalid handlebars syntax should fail");

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn default_html_hot_path_matches_handlebars_output() {
    use super::RemovePeriodsHelper;
    use handlebars::Handlebars;

    let options = GenerateWebfontsOptions {
        css: Some(true),
        codepoints: Some(HashMap::from([
            ("add".to_owned(), 0xE001u32),
            ("remove".to_owned(), 0xE002u32),
        ])),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_dest: Some("artifacts/iconfont.html".to_owned()),
        html_template: Some(format!("{}/templates/html.hbs", env!("CARGO_MANIFEST_DIR"))),
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        start_codepoint: Some(0xE001),
        template_options: Some(serde_json::Map::from_iter([
            (
                "baseSelector".to_owned(),
                serde_json::Value::String(".icon".to_owned()),
            ),
            (
                "classPrefix".to_owned(),
                serde_json::Value::String("icon-".to_owned()),
            ),
        ])),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };
    let options = resolve_options(options);
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();

    // Build HTML context with embedded CSS styles
    let html_ctx = super::build_html_context(&options, &shared, &source_files, None).unwrap();

    // Render via Handlebars (template path is set)
    let handlebars_output = {
        let source = std::fs::read_to_string(options.html_template.as_ref().unwrap()).unwrap();
        let mut registry = Handlebars::new();
        registry.register_helper("removePeriods", Box::new(RemovePeriodsHelper));
        registry.render_template(&source, &html_ctx).unwrap()
    };

    // Render via hot path (no template = default)
    let hot_path_output = super::render_default_html(&html_ctx);

    assert_eq!(
        hot_path_output, handlebars_output,
        "HTML hot path output must match Handlebars output"
    );
}

#[test]
fn render_html_with_hbs_context_matches_direct_render_for_default_template() {
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_html_context(&options, &shared, &source_files, None).unwrap();
    let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

    let direct = render_html_with_context(&options, None, &ctx).unwrap();
    let via_hbs = super::render_html_with_hbs_context(None, &hbs_ctx, &ctx).unwrap();

    assert_eq!(
        via_hbs, direct,
        "render_html_with_hbs_context must match render_html_with_context for default template"
    );
}

#[test]
fn render_html_with_hbs_context_matches_direct_render_for_custom_template() {
    let template_path = write_temp_template(
        "native-html-hbs-ctx",
        "<html><body>{{fontName}} {{{styles}}}</body></html>",
    );
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_template: Some(template_path),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_html_context(&options, &shared, &source_files, None).unwrap();
    let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();
    let registry = build_html_registry(&options).unwrap();

    let direct = render_html_with_context(&options, registry.as_ref(), &ctx).unwrap();
    let via_hbs = super::render_html_with_hbs_context(registry.as_ref(), &hbs_ctx, &ctx).unwrap();

    assert_eq!(
        via_hbs, direct,
        "render_html_with_hbs_context must match render_html_with_context for custom template"
    );
}

#[test]
fn render_html_with_styles_swap_via_hbs_context_matches_manual_rewrite() {
    let template_path = write_temp_template(
        "native-html-styles-swap",
        "<html><style>{{{styles}}}</style>{{fontName}}</html>",
    );
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_template: Some(template_path),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_html_context(&options, &shared, &source_files, None).unwrap();
    let registry = build_html_registry(&options).unwrap().unwrap();

    let new_styles = "body { font-family: custom; }";

    // Manual: clone Map, replace styles, render
    let mut manual_ctx = ctx.clone();
    manual_ctx.insert(
        "styles".to_owned(),
        serde_json::Value::String(new_styles.to_owned()),
    );
    let expected = registry.render("html", &manual_ctx).unwrap();

    // Optimized: clone hbs Context, swap styles field, render_with_context
    let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();
    let mut swapped = hbs_ctx.clone();
    swapped
        .data_mut()
        .as_object_mut()
        .expect("context should be object")
        .insert(
            "styles".to_owned(),
            serde_json::Value::String(new_styles.to_owned()),
        );
    let actual = registry.render_with_context("html", &swapped).unwrap();

    assert_eq!(
        actual, expected,
        "HTML Context swap must produce identical output to manual Map rewrite"
    );
}

#[test]
fn html_styles_in_place_mutation_restores_original_and_works_repeatedly() {
    let template_path = write_temp_template(
        "native-html-styles-mutate",
        "<style>{{{styles}}}</style>{{fontName}}",
    );
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(true),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(true),
        html_template: Some(template_path),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    let shared = SharedTemplateData::new(&options, &source_files).unwrap();
    let ctx = build_html_context(&options, &shared, &source_files, None).unwrap();
    let registry = build_html_registry(&options).unwrap().unwrap();
    let mut hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

    let original_styles = hbs_ctx.data().as_object().unwrap().get("styles").cloned();

    // First mutation
    let styles_a = "body { color: red; }";
    let obj = hbs_ctx.data_mut().as_object_mut().unwrap();
    let prev = obj.insert(
        "styles".to_owned(),
        serde_json::Value::String(styles_a.to_owned()),
    );
    let result_a = registry.render_with_context("html", &hbs_ctx).unwrap();
    // Restore
    match prev {
        Some(v) => {
            hbs_ctx
                .data_mut()
                .as_object_mut()
                .unwrap()
                .insert("styles".to_owned(), v);
        }
        None => {
            hbs_ctx.data_mut().as_object_mut().unwrap().remove("styles");
        }
    }

    // Second mutation with different styles
    let styles_b = "body { color: blue; }";
    let obj = hbs_ctx.data_mut().as_object_mut().unwrap();
    let prev = obj.insert(
        "styles".to_owned(),
        serde_json::Value::String(styles_b.to_owned()),
    );
    let result_b = registry.render_with_context("html", &hbs_ctx).unwrap();
    // Restore
    match prev {
        Some(v) => {
            hbs_ctx
                .data_mut()
                .as_object_mut()
                .unwrap()
                .insert("styles".to_owned(), v);
        }
        None => {
            hbs_ctx.data_mut().as_object_mut().unwrap().remove("styles");
        }
    }

    assert!(result_a.contains(styles_a));
    assert!(result_b.contains(styles_b));
    assert_ne!(result_a, result_b);

    // Verify restoration
    let restored = hbs_ctx.data().as_object().unwrap().get("styles").cloned();
    assert_eq!(
        restored, original_styles,
        "original styles should be restored after mutations"
    );
}

#[test]
fn build_html_registry_rejects_invalid_template_syntax() {
    let template_path = write_temp_template("native-html-invalid-compile", "{{#if}}");
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![crate::test_helpers::webfont_fixture("add.svg")],
        html: Some(false),
        html_template: Some(template_path),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    });

    match build_html_registry(&options) {
        Err(error) => {
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert!(
                error
                    .to_string()
                    .contains("Failed to compile HTML template"),
                "error should mention HTML template compilation: {error}"
            );
        }
        Ok(_) => panic!(
            "invalid handlebars syntax should always fail — generateHtml() may be called later"
        ),
    }
}
