use super::*;

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::test_helpers::write_temp_template;

fn generate_with_templates(
    paths: Vec<String>,
    incremental: bool,
    css_template: Option<String>,
    html_template: Option<String>,
) -> GenerateWebfontsResult {
    generate_with_templates_and_options(paths, incremental, css_template, html_template, None)
}

fn generate_with_templates_and_options(
    paths: Vec<String>,
    incremental: bool,
    css_template: Option<String>,
    html_template: Option<String>,
    template_options: Option<Map<String, Value>>,
) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(css_template.is_some()),
        css_template,
        dest: "artifacts".to_owned(),
        files: paths,
        html: Some(html_template.is_some()),
        html_template,
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options(false)),
        ligature: Some(false),
        incremental: Some(incremental),
        template_options,
        write_files: Some(false),
        types: Some(vec![FontType::Woff2]),
        ..Default::default()
    })
    .unwrap();
    let source_files = load(&resolved.files);
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    generate_webfonts_sync(resolved, source_files).unwrap()
}

#[test]
fn regenerate_reuses_provided_url_css_on_content_edit() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let urls = HashMap::from([(FontType::Woff2, "/static/icons.woff2".to_owned())]);

    let mut result = generate_with_css(vec![a.clone(), b.clone()], true);
    let before = result.generate_css_pure(Some(urls.clone())).unwrap();

    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();
    let after = result.generate_css_pure(Some(urls.clone())).unwrap();

    assert_eq!(
        before, after,
        "provided-url CSS is independent of the font bytes"
    );
    let fresh = generate_with_css(vec![a, b], false);
    assert_eq!(after, fresh.generate_css_pure(Some(urls)).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[cfg(feature = "napi")]
#[test]
fn async_snapshot_carries_reusable_render_cache() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let template = write_temp_template("css-static-async", ".icon { font-family: {{fontName}}; }");
    let result = generate_with_templates(vec![a.clone(), b.clone()], true, Some(template), None);
    result.generate_css_pure(None).unwrap();

    let state = result.take_regeneration_state().unwrap();
    let mut replacement = result.snapshot_for_regeneration(state);
    write_icon(&dir, "b", D_CHANGED);
    replacement
        .regenerate(&[a, b.clone()], &[(b, GlyphChange::Changed { name: None })])
        .unwrap();

    assert!(replacement.has_carried_css_no_urls_for_test());
    std::fs::remove_dir_all(&dir).ok();
}

#[cfg(feature = "napi")]
#[test]
fn consecutive_async_snapshots_invalidate_codepoint_dependent_css() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let template = write_temp_template("css-codepoints-async", "{{codepoints.a}}");
    let result = generate_with_templates(vec![a.clone(), b.clone()], true, Some(template), None);
    result.generate_css_pure(None).unwrap();

    let state = result.take_regeneration_state().unwrap();
    let mut first = result.snapshot_for_regeneration(state);
    write_icon(&dir, "b", D_CHANGED);
    first
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();
    assert!(first.has_carried_css_no_urls_for_test());

    let state = first.take_regeneration_state().unwrap();
    let mut second = first.snapshot_for_regeneration(state);
    let c = write_icon(&dir, "c", D3);
    second
        .regenerate(
            &[a, b, c.clone()],
            &[(c, GlyphChange::Added { name: None })],
        )
        .unwrap();
    assert!(!second.has_carried_css_no_urls_for_test());
    std::fs::remove_dir_all(&dir).ok();
}

#[cfg(feature = "napi")]
#[test]
fn consecutive_async_snapshots_invalidate_name_dependent_html() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let template = write_temp_template("html-names-async", "{{names.0}}");
    let result = generate_with_templates(vec![a.clone(), b.clone()], true, None, Some(template));
    result.generate_html_pure(None).unwrap();

    let state = result.take_regeneration_state().unwrap();
    let mut first = result.snapshot_for_regeneration(state);
    write_icon(&dir, "b", D_CHANGED);
    first
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();
    assert!(first.has_carried_html_no_urls_for_test());

    let state = first.take_regeneration_state().unwrap();
    let mut second = first.snapshot_for_regeneration(state);
    second
        .regenerate(
            &[a, b.clone()],
            &[(
                b,
                GlyphChange::Changed {
                    name: Some("renamed".to_owned()),
                },
            )],
        )
        .unwrap();
    assert!(!second.has_carried_html_no_urls_for_test());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_rerenders_default_css_on_content_edit() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);

    let mut result = generate_with_css(vec![a.clone(), b.clone()], true);
    let before = result.generate_css_pure(None).unwrap();

    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();
    let after = result.generate_css_pure(None).unwrap();

    assert_ne!(
        before, after,
        "default CSS embeds the source hash, which changed"
    );
    let fresh = generate_with_css(vec![a, b], false);
    assert_eq!(after, fresh.generate_css_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_rerenders_css_on_rename() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let urls = HashMap::from([(FontType::Woff2, "/static/icons.woff2".to_owned())]);

    let mut result = generate_with_css(vec![a.clone(), b.clone()], true);
    let before = result.generate_css_pure(Some(urls.clone())).unwrap();

    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(
                b.clone(),
                GlyphChange::Changed {
                    name: Some("renamed".to_owned()),
                },
            )],
        )
        .unwrap();
    let after = result.generate_css_pure(Some(urls)).unwrap();

    assert_ne!(before, after, "a renamed glyph must re-render");
    assert!(
        after.contains("renamed"),
        "new glyph name should appear in the CSS"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_carries_css_for_template_that_ignores_changed_glyph_data() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let css_template = write_temp_template("css-font-name-only", "{{fontName}}\n");

    let mut result =
        generate_with_templates(vec![a.clone(), b.clone()], true, Some(css_template), None);
    let before = result.generate_css_pure(None).unwrap();
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(c.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert!(
        result.has_carried_css_no_urls_for_test(),
        "CSS that only reads stable fields should be carried across glyph set changes"
    );
    assert_eq!(before, result.generate_css_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_css_for_template_that_reads_codepoints() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let css_template = write_temp_template(
        "css-codepoints",
        "{{#each codepoints}}{{@key}}={{this}};{{/each}}\n",
    );

    let mut result =
        generate_with_templates(vec![a.clone(), b.clone()], true, Some(css_template), None);
    result.generate_css_pure(None).unwrap();
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(c.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert!(
        !result.has_carried_css_no_urls_for_test(),
        "CSS that reads codepoints must be re-rendered when codepoints change"
    );
    assert!(result.generate_css_pure(None).unwrap().contains("c="));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_carries_html_for_template_that_ignores_names_and_styles() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let html_template = write_temp_template("html-font-name-only", "<h1>{{fontName}}</h1>\n");

    let mut result =
        generate_with_templates(vec![a.clone(), b.clone()], true, None, Some(html_template));
    let before = result.generate_html_pure(None).unwrap();
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(c.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert!(
        result.has_carried_html_no_urls_for_test(),
        "HTML that only reads stable fields should be carried across glyph set changes"
    );
    assert_eq!(before, result.generate_html_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_html_for_template_that_reads_names() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let html_template = write_temp_template("html-names", "{{#each names}}{{this}};{{/each}}\n");

    let mut result =
        generate_with_templates(vec![a.clone(), b.clone()], true, None, Some(html_template));
    result.generate_html_pure(None).unwrap();
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(c.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert!(
        !result.has_carried_html_no_urls_for_test(),
        "HTML that reads names must be re-rendered when names change"
    );
    assert!(result.generate_html_pure(None).unwrap().contains("c;"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_html_for_template_that_reads_root_styles() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let html_template =
        write_temp_template("html-root-styles", "<style>{{{@root.styles}}}</style>\n");

    let mut result = generate_with_templates(
        vec![a.clone(), b.clone()],
        true,
        None,
        Some(html_template.clone()),
    );
    let before = result.generate_html_pure(None).unwrap();
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();
    let after = result.generate_html_pure(None).unwrap();

    assert_ne!(before, after, "HTML styles changed with the font hash");
    assert!(
        !result.has_carried_html_no_urls_for_test(),
        "HTML that reads @root.styles must be re-rendered when styles change"
    );
    let fresh = generate_with_templates(vec![a, b], false, None, Some(html_template));
    assert_eq!(after, fresh.generate_html_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_html_for_template_that_reads_trimmed_styles() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let html_template = write_temp_template("html-trimmed-styles", "<style>{{~styles~}}</style>\n");

    let mut result = generate_with_templates(
        vec![a.clone(), b.clone()],
        true,
        None,
        Some(html_template.clone()),
    );
    let before = result.generate_html_pure(None).unwrap();
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();
    let after = result.generate_html_pure(None).unwrap();

    assert_ne!(before, after, "HTML styles changed with the font hash");
    assert!(
        !result.has_carried_html_no_urls_for_test(),
        "HTML that reads {{~styles~}} must be re-rendered when styles change"
    );
    let fresh = generate_with_templates(vec![a, b], false, None, Some(html_template));
    assert_eq!(after, fresh.generate_html_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_html_for_lookup_subexpression_template() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let html_template = write_temp_template(
        "html-lookup-names",
        "{{#each (lookup this field)}}{{this}};{{/each}}\n",
    );
    let template_options =
        Map::from_iter([("field".to_owned(), Value::String("names".to_owned()))]);

    let mut result = generate_with_templates_and_options(
        vec![a.clone(), b.clone()],
        true,
        None,
        Some(html_template.clone()),
        Some(template_options.clone()),
    );
    result.generate_html_pure(None).unwrap();
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(c.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert!(
        !result.has_carried_html_no_urls_for_test(),
        "HTML with lookup subexpressions must not carry render cache"
    );
    let after = result.generate_html_pure(None).unwrap();
    assert!(after.contains("c;"));
    let fresh = generate_with_templates_and_options(
        vec![a, b, c],
        false,
        None,
        Some(html_template),
        Some(template_options),
    );
    assert_eq!(after, fresh.generate_html_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_html_for_block_param_template() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let html_template = write_temp_template(
        "html-block-param-names",
        "{{#with this as |ctx|}}{{#each ctx.names}}{{this}};{{/each}}{{/with}}\n",
    );

    let mut result = generate_with_templates(
        vec![a.clone(), b.clone()],
        true,
        None,
        Some(html_template.clone()),
    );
    result.generate_html_pure(None).unwrap();
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(c.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert!(
        !result.has_carried_html_no_urls_for_test(),
        "HTML with block-param aliases must not carry render cache"
    );
    let after = result.generate_html_pure(None).unwrap();
    assert!(after.contains("c;"));
    let fresh = generate_with_templates(vec![a, b, c], false, None, Some(html_template));
    assert_eq!(after, fresh.generate_html_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_html_for_whole_context_template() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let html_template = write_temp_template("html-whole-context", "{{this}}\n");

    let mut result = generate_with_templates(
        vec![a.clone(), b.clone()],
        true,
        None,
        Some(html_template.clone()),
    );
    result.generate_html_pure(None).unwrap();
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(c.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert!(
        !result.has_carried_html_no_urls_for_test(),
        "HTML with whole-context reads must not carry render cache"
    );
    let after = result.generate_html_pure(None).unwrap();
    assert!(after.contains("c"));
    let fresh = generate_with_templates(vec![a, b, c], false, None, Some(html_template));
    assert_eq!(after, fresh.generate_html_pure(None).unwrap());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_drops_dynamic_css_template_cache_conservatively() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let css_template = write_temp_template("css-dynamic", "{{lookup codepoints \"a\"}}\n");

    let mut result =
        generate_with_templates(vec![a.clone(), b.clone()], true, Some(css_template), None);
    result.generate_css_pure(None).unwrap();
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();

    assert!(
        !result.has_carried_css_no_urls_for_test(),
        "dynamic template access should not be carried across regenerates"
    );
    std::fs::remove_dir_all(&dir).ok();
}
