use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::input::LoadedSvgFile;
use crate::input::{finalize_generate_webfonts_options, resolve_generate_webfonts_options};
use crate::types::{FontOutputs, FontType, GenerateWebfontsOptions, GenerateWebfontsResult};

fn build_result(template: Option<&str>) -> GenerateWebfontsResult {
    build_result_with_templates(template, None)
}

fn build_result_with_templates(
    css_template: Option<&str>,
    html_template: Option<&str>,
) -> GenerateWebfontsResult {
    let fixture = crate::test_helpers::webfont_fixture("add.svg");
    let css_template = css_template
        .map(|content| crate::test_helpers::write_temp_template("render-cache-css", content));
    let html_template = html_template
        .map(|content| crate::test_helpers::write_temp_template("render-cache-html", content));

    let options = GenerateWebfontsOptions {
        css: Some(true),
        css_template,
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        dest: "artifacts".to_owned(),
        files: vec![fixture],
        html: Some(html_template.is_some()),
        html_template,
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Svg]),
        start_codepoint: Some(0xE001),
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    };

    let mut resolved = resolve_generate_webfonts_options(options).unwrap();
    let source_files: Vec<LoadedSvgFile> = resolved
        .files
        .iter()
        .map(|path| LoadedSvgFile {
            contents: std::fs::read_to_string(path).unwrap().into(),
            glyph_name: std::path::Path::new(path)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            path: path.clone(),
        })
        .collect();
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();

    GenerateWebfontsResult {
        cached: std::sync::OnceLock::new(),
        carried_render: None,
        css_context: None,
        fonts: FontOutputs::default(),
        html_context: None,
        options: Arc::new(resolved),
        regeneration_state: Arc::new(Mutex::new(None)),
        source_files: Arc::new(source_files),
    }
}

#[test]
fn generate_css_returns_cached_result_on_repeated_calls_without_urls() {
    let result = build_result(None);

    let first = result.generate_css_pure(None).unwrap();
    let second = result.generate_css_pure(None).unwrap();

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

#[test]
fn generate_css_returns_cached_result_on_repeated_calls_with_same_urls() {
    let result = build_result(None);
    let urls = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);

    let first = result.generate_css_pure(Some(urls.clone())).unwrap();
    let second = result.generate_css_pure(Some(urls)).unwrap();

    assert_eq!(first, second);
    assert!(first.contains("/a.svg"));
}

#[test]
fn generate_css_returns_different_result_for_different_urls() {
    let result = build_result(None);
    let urls_a = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);
    let urls_b = HashMap::from([(FontType::Svg, "/b.svg".to_owned())]);

    let result_a = result.generate_css_pure(Some(urls_a)).unwrap();
    let result_b = result.generate_css_pure(Some(urls_b)).unwrap();

    assert_ne!(result_a, result_b);
    assert!(result_a.contains("/a.svg"));
    assert!(result_b.contains("/b.svg"));
}

#[test]
fn generate_css_cache_updates_when_urls_change() {
    let result = build_result(None);
    let urls_a = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);
    let urls_b = HashMap::from([(FontType::Svg, "/b.svg".to_owned())]);

    let first_a = result.generate_css_pure(Some(urls_a.clone())).unwrap();
    let first_b = result.generate_css_pure(Some(urls_b)).unwrap();
    let second_a = result.generate_css_pure(Some(urls_a)).unwrap();

    assert_eq!(
        first_a, second_a,
        "returning to original urls should produce same result"
    );
    assert_ne!(first_a, first_b);
}

#[test]
fn generate_css_cache_works_with_custom_template() {
    let result = build_result(Some("@font-face { src: {{{src}}}; }"));
    let urls = HashMap::from([(FontType::Svg, "/cached.svg".to_owned())]);

    let first = result.generate_css_pure(Some(urls.clone())).unwrap();
    let second = result.generate_css_pure(Some(urls)).unwrap();

    assert_eq!(first, second);
    assert!(first.contains("/cached.svg"));
}

#[test]
fn generate_html_restores_styles_after_render_error() {
    let result = build_result_with_templates(None, Some("{{removePeriods}}"));
    let cached = result.get_cached_io().unwrap();
    let original_styles = cached
        .html_hbs_context
        .lock()
        .unwrap()
        .data()
        .get("styles")
        .cloned();

    let error = result
        .generate_html_pure(Some(HashMap::from([(
            FontType::Svg,
            "/error.svg".to_owned(),
        )])))
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(
        cached
            .html_hbs_context
            .lock()
            .unwrap()
            .data()
            .get("styles")
            .cloned(),
        original_styles
    );
}

#[test]
fn generate_css_no_urls_and_with_urls_are_independent_caches() {
    let result = build_result(None);
    let urls = HashMap::from([(FontType::Svg, "/custom.svg".to_owned())]);

    let no_urls = result.generate_css_pure(None).unwrap();
    let with_urls = result.generate_css_pure(Some(urls)).unwrap();
    let no_urls_again = result.generate_css_pure(None).unwrap();

    assert_eq!(
        no_urls, no_urls_again,
        "no-urls cache should survive a with-urls call"
    );
    assert_ne!(no_urls, with_urls);
}

#[test]
fn generate_css_with_urls_returns_no_urls_result_when_template_does_not_use_src() {
    let result = build_result(Some(".icon { font-family: {{fontName}}; }"));
    let urls = HashMap::from([(FontType::Svg, "/should-not-appear.svg".to_owned())]);

    let no_urls = result.generate_css_pure(None).unwrap();
    let with_urls = result.generate_css_pure(Some(urls)).unwrap();

    assert_eq!(
        no_urls, with_urls,
        "template without {{src}} should ignore urls"
    );
    assert!(!with_urls.contains("/should-not-appear.svg"));
    assert!(
        with_urls.contains("iconfont"),
        "should still render the template"
    );
}

#[test]
fn generate_html_with_urls_returns_no_urls_result_when_css_template_does_not_use_src() {
    let result = build_result(Some(".icon { font-family: {{fontName}}; }"));
    let urls = HashMap::from([(FontType::Svg, "/should-not-appear.svg".to_owned())]);

    let no_urls = result.generate_html_pure(None).unwrap();
    let with_urls = result.generate_html_pure(Some(urls)).unwrap();

    assert_eq!(
        no_urls, with_urls,
        "CSS template without {{src}} means HTML is also unaffected by urls"
    );
}

#[test]
fn generate_css_without_urls_produces_valid_css_using_css_fonts_url() {
    let result = build_result(None);

    let css = result.generate_css_pure(None).unwrap();

    assert!(
        css.contains("@font-face"),
        "should contain @font-face declaration"
    );
    assert!(css.contains("font-family:"), "should contain font-family");
    assert!(
        css.contains("iconfont.svg?"),
        "should use font name in URL with hash"
    );
    assert!(
        css.contains("format(\"svg\")"),
        "should contain format declaration"
    );
    assert!(
        css.contains("content:"),
        "should contain codepoint content rules"
    );
}

#[test]
fn generate_css_with_urls_replaces_default_urls_in_src() {
    let result = build_result(None);
    let urls = HashMap::from([(FontType::Svg, "/cdn/icons.svg".to_owned())]);

    let css = result.generate_css_pure(Some(urls)).unwrap();

    assert!(
        css.contains("/cdn/icons.svg"),
        "custom URL should appear in output"
    );
    assert!(
        !css.contains("iconfont.svg?"),
        "default hash-based URL should not appear"
    );
    assert!(
        css.contains("format(\"svg\")"),
        "format should still be present"
    );
}

#[test]
fn generate_html_without_urls_produces_valid_html() {
    let result = build_result(None);

    let html = result.generate_html_pure(None).unwrap();

    assert!(
        html.contains("<!DOCTYPE html>"),
        "should be a full HTML document"
    );
    assert!(html.contains("iconfont"), "should contain font name");
    assert!(html.contains("icon-add"), "should contain icon class name");
}

#[test]
fn generate_html_with_urls_embeds_css_using_custom_urls() {
    let result = build_result(None);
    let urls = HashMap::from([(FontType::Svg, "/cdn/icons.svg".to_owned())]);

    let html = result.generate_html_pure(Some(urls)).unwrap();

    assert!(
        html.contains("/cdn/icons.svg"),
        "custom URL should appear in embedded CSS"
    );
    assert!(
        html.contains("icon-add"),
        "should still contain icon class name"
    );
}

#[test]
fn generate_html_cache_returns_same_result_for_same_urls() {
    let result = build_result(None);
    let urls = HashMap::from([(FontType::Svg, "/cached.svg".to_owned())]);

    let first = result.generate_html_pure(Some(urls.clone())).unwrap();
    let second = result.generate_html_pure(Some(urls)).unwrap();

    assert_eq!(first, second);
    assert!(first.contains("/cached.svg"));
}

#[test]
fn generate_html_cache_returns_different_result_for_different_urls() {
    let result = build_result(None);
    let urls_a = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);
    let urls_b = HashMap::from([(FontType::Svg, "/b.svg".to_owned())]);

    let result_a = result.generate_html_pure(Some(urls_a)).unwrap();
    let result_b = result.generate_html_pure(Some(urls_b)).unwrap();

    assert_ne!(result_a, result_b);
    assert!(result_a.contains("/a.svg"));
    assert!(result_b.contains("/b.svg"));
}

#[test]
fn generate_custom_html_with_urls_updates_styles_without_changing_default_render() {
    let result = build_result_with_templates(None, Some("<style>{{{styles}}}</style>"));
    let default = result.generate_html_pure(None).unwrap();
    let urls_a = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);
    let urls_b = HashMap::from([(FontType::Svg, "/b.svg".to_owned())]);

    let result_a = result.generate_html_pure(Some(urls_a.clone())).unwrap();
    let result_b = result.generate_html_pure(Some(urls_b)).unwrap();

    assert!(result_a.contains("/a.svg"));
    assert!(result_b.contains("/b.svg"));
    assert_eq!(result_a, result.generate_html_pure(Some(urls_a)).unwrap());
    assert_eq!(default, result.generate_html_pure(None).unwrap());
}

/// Build a result with multiple font types (svg + woff2) for testing partial URL overrides.
fn build_multi_type_result() -> GenerateWebfontsResult {
    let fixture = crate::test_helpers::webfont_fixture("add.svg");
    let options = GenerateWebfontsOptions {
        css: Some(true),
        codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
        dest: "artifacts".to_owned(),
        files: vec![fixture],
        html: Some(true),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        order: Some(vec![FontType::Woff2, FontType::Svg]),
        start_codepoint: Some(0xE001),
        types: Some(vec![FontType::Svg, FontType::Woff2]),
        ..Default::default()
    };

    let mut resolved = resolve_generate_webfonts_options(options).unwrap();
    let source_files: Vec<LoadedSvgFile> = resolved
        .files
        .iter()
        .map(|path| LoadedSvgFile {
            contents: std::fs::read_to_string(path).unwrap().into(),
            glyph_name: std::path::Path::new(path)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            path: path.clone(),
        })
        .collect();
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();

    GenerateWebfontsResult {
        cached: std::sync::OnceLock::new(),
        carried_render: None,
        css_context: None,
        fonts: FontOutputs::default(),
        html_context: None,
        options: Arc::new(resolved),
        regeneration_state: Arc::new(Mutex::new(None)),
        source_files: Arc::new(source_files),
    }
}

#[test]
fn generate_css_partial_urls_uses_empty_string_for_missing_types() {
    let result = build_multi_type_result();
    // Override only woff2, leave svg un-provided -- matches upstream behavior
    let urls = HashMap::from([(FontType::Woff2, "/cdn/font.woff2".to_owned())]);

    let css = result.generate_css_pure(Some(urls)).unwrap();

    assert!(
        css.contains("/cdn/font.woff2"),
        "overridden URL should appear"
    );
    assert!(
        !css.contains("iconfont.svg?"),
        "non-overridden type should not have default hash-based URL"
    );
    assert!(
        css.contains("url(\"#iconfont\")"),
        "non-overridden SVG type should produce empty base URL (upstream compat)"
    );
}

#[test]
fn generate_html_partial_urls_uses_empty_string_for_missing_types() {
    let result = build_multi_type_result();
    let urls = HashMap::from([(FontType::Woff2, "/cdn/font.woff2".to_owned())]);

    let html = result.generate_html_pure(Some(urls)).unwrap();

    assert!(
        html.contains("/cdn/font.woff2"),
        "overridden URL should appear in HTML"
    );
    assert!(
        !html.contains("iconfont.svg?"),
        "non-overridden type should not have default hash-based URL in HTML"
    );
}
