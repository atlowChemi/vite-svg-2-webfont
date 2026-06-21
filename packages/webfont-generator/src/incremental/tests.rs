use std::collections::HashMap;
use std::path::Path;

use serde_json::{Map, Value};

use crate::test_helpers::write_temp_template;
use crate::types::{FontType, GenerateWebfontsResult, GlyphChange, LoadedSvgFile};
use crate::{FormatOptions, GenerateWebfontsOptions, TtfFormatOptions};
use crate::{
    finalize_generate_webfonts_options, generate_webfonts_sync, resolve_generate_webfonts_options,
};

const D1: &str = "M2 2 L22 2 L22 22 Z";
const D2: &str = "M2 2 L22 2 L12 22 Z";
const D3: &str = "M4 4 L20 4 L20 20 L4 20 Z";
const D_CHANGED: &str = "M0 0 L24 0 L24 24 Z";
const TEST_TTF_TIMESTAMP: i64 = 1_700_000_000;

fn stable_format_options() -> FormatOptions {
    FormatOptions {
        ttf: Some(TtfFormatOptions {
            copyright: None,
            description: None,
            ts: Some(TEST_TTF_TIMESTAMP),
            url: None,
            version: None,
        }),
        ..Default::default()
    }
}

fn temp_dir() -> std::path::PathBuf {
    // Process id + a monotonic counter so parallel tests never collide on the same dir.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("recalc-ut-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_icon(dir: &Path, name: &str, d: &str) -> String {
    let path = dir.join(format!("{name}.svg"));
    std::fs::write(
        &path,
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"{d}\"/></svg>"),
    )
    .unwrap();
    path.to_string_lossy().into_owned()
}

fn load(paths: &[String]) -> Vec<LoadedSvgFile> {
    paths
        .iter()
        .map(|path| LoadedSvgFile {
            contents: std::fs::read_to_string(path).unwrap(),
            glyph_name: Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned(),
            path: path.clone(),
        })
        .collect()
}

fn generate(paths: Vec<String>, incremental: bool) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: paths,
        html: Some(false),
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options()),
        ligature: Some(false),
        incremental: Some(incremental),
        // These tests assert in-memory parity; don't touch the disk on regenerate.
        write_files: Some(false),
        types: Some(vec![
            FontType::Svg,
            FontType::Ttf,
            FontType::Eot,
            FontType::Woff,
            FontType::Woff2,
        ]),
        ..Default::default()
    })
    .unwrap();
    let source_files = load(&resolved.files);
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    generate_webfonts_sync(resolved, source_files).unwrap()
}

fn assert_same(actual: &GenerateWebfontsResult, expected: &GenerateWebfontsResult) {
    assert_eq!(actual.svg_string(), expected.svg_string(), "svg mismatch");
    assert_eq!(actual.ttf_bytes(), expected.ttf_bytes(), "ttf mismatch");
    assert_eq!(actual.eot_bytes(), expected.eot_bytes(), "eot mismatch");
    assert_eq!(actual.woff_bytes(), expected.woff_bytes(), "woff mismatch");
    assert_eq!(
        actual.woff2_bytes(),
        expected.woff2_bytes(),
        "woff2 mismatch"
    );
}

#[test]
fn regenerate_after_content_change_matches_fresh() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![a.clone(), b.clone(), c.clone()], true);
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();

    assert_same(&result, &generate(vec![a, b, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_after_add_matches_fresh() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);

    let mut result = generate(vec![a.clone(), b.clone()], true);
    let c = write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(
                c.clone(),
                GlyphChange::Added {
                    name: Some("c".to_owned()),
                },
            )],
        )
        .unwrap();

    assert_same(&result, &generate(vec![a, b, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_after_mid_order_add_matches_fresh() {
    let dir = temp_dir();
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![b.clone(), c.clone()], true);
    let a = write_icon(&dir, "a", D1);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(a.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();

    assert_same(&result, &generate(vec![a, b, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_after_remove_matches_fresh() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![a.clone(), b.clone(), c.clone()], true);
    result
        .regenerate(&[a.clone(), c.clone()], &[(b, GlyphChange::Removed)])
        .unwrap();

    assert_same(&result, &generate(vec![a, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_all_after_content_change_matches_fresh() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![a.clone(), b.clone(), c.clone()], true);
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate_all(&[a.clone(), b.clone(), c.clone()])
        .unwrap();

    assert_same(&result, &generate(vec![a, b, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_all_after_add_and_remove_matches_fresh() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![a.clone(), b], true);
    result.regenerate_all(&[a.clone(), c.clone()]).unwrap();

    assert_same(&result, &generate(vec![a, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_all_noop_returns_before_parsing() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let mut result = generate(vec![a.clone(), b.clone()], true);
    let before = result.glyph_cache.as_ref().unwrap().parse_count;

    result.regenerate_all(&[a, b]).unwrap();

    assert_eq!(result.glyph_cache.as_ref().unwrap().parse_count, before);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_all_without_incremental_errors() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let mut result = generate(vec![a.clone()], false);
    let error = result.regenerate_all(&[a]).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_without_incremental_errors() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let mut result = generate(vec![a.clone()], false);
    let changes = [(a.clone(), GlyphChange::Changed { name: None })];
    let error = result
        .regenerate(std::slice::from_ref(&a), &changes)
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn non_incremental_results_do_not_retain_glyph_cache() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let result = generate(vec![a], false);

    assert!(
        result.glyph_cache.is_none(),
        "one-shot builds must not retain parsed glyph geometry"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn incremental_results_seed_glyph_cache_for_active_files() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let result = generate(vec![a, b], true);
    let cache = result.glyph_cache.as_ref().unwrap();

    assert_eq!(cache.entries.len(), 2);
    assert_eq!(cache.content_hashes.len(), 2);
    assert_eq!(cache.by_content_hash.len(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_noop_changed_event_returns_before_parsing() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let mut result = generate(vec![a.clone(), b.clone()], true);
    let before = result.glyph_cache.as_ref().unwrap().parse_count;

    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b, GlyphChange::Changed { name: None })],
        )
        .unwrap();

    assert_eq!(
        result.glyph_cache.as_ref().unwrap().parse_count,
        before,
        "unchanged watcher events should return before SVG parsing"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_added_duplicate_reuses_content_addressed_cache() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let mut result = generate(vec![a.clone(), b.clone()], true);
    let c = write_icon(&dir, "c", D1);
    let before = result.glyph_cache.as_ref().unwrap().parse_count;

    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(
                c.clone(),
                GlyphChange::Added {
                    name: Some("c".to_owned()),
                },
            )],
        )
        .unwrap();

    assert_eq!(
        result.glyph_cache.as_ref().unwrap().parse_count,
        before,
        "added files with SVG bytes already in the cache should not be parsed again"
    );
    assert_same(&result, &generate(vec![a, b, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_remove_prunes_inactive_cache_entries() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);
    let mut result = generate(vec![a.clone(), b.clone(), c.clone()], true);

    result
        .regenerate(&[a.clone(), c.clone()], &[(b, GlyphChange::Removed)])
        .unwrap();

    let cache = result.glyph_cache.as_ref().unwrap();
    assert_eq!(cache.entries.len(), 2);
    assert_eq!(cache.content_hashes.len(), 2);
    assert_eq!(cache.by_content_hash.len(), 2);
    assert!(cache.entries.contains_key(&a));
    assert!(cache.entries.contains_key(&c));
    assert_same(&result, &generate(vec![a, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_add_remove_cycles_do_not_grow_cache() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let mut result = generate(vec![a.clone(), b.clone()], true);

    for index in 0..5 {
        let name = format!("extra-{index}");
        let path_data = format!("M{index} {index} L24 0 L24 24 Z");
        let extra = write_icon(&dir, &name, &path_data);
        let with_extra = vec![a.clone(), b.clone(), extra.clone()];
        result
            .regenerate(
                &with_extra,
                &[(extra.clone(), GlyphChange::Added { name: None })],
            )
            .unwrap();

        let cache = result.glyph_cache.as_ref().unwrap();
        assert_eq!(cache.entries.len(), 3);
        assert_eq!(cache.content_hashes.len(), 3);
        assert_eq!(cache.by_content_hash.len(), 3);

        result
            .regenerate(&[a.clone(), b.clone()], &[(extra, GlyphChange::Removed)])
            .unwrap();

        let cache = result.glyph_cache.as_ref().unwrap();
        assert_eq!(cache.entries.len(), 2);
        assert_eq!(cache.content_hashes.len(), 2);
        assert_eq!(cache.by_content_hash.len(), 2);
        assert!(cache.entries.contains_key(&a));
        assert!(cache.entries.contains_key(&b));
    }

    assert_same(&result, &generate(vec![a, b], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_with_context_callback_state_errors() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let mut result = generate(vec![a.clone()], true);
    result.css_context = Some(Default::default());

    let error = result
        .regenerate(&[a], &[])
        .expect_err("regenerate must reject pre-mutated callback contexts");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("cssContext/htmlContext"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_failure_preserves_incremental_state_for_retry() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = dir.join("c.svg").to_string_lossy().into_owned();
    let mut result = generate(vec![a.clone(), b.clone()], true);
    let before = result.svg_string().unwrap().to_owned();

    let error = result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(
                c.clone(),
                GlyphChange::Added {
                    name: Some("c".to_owned()),
                },
            )],
        )
        .expect_err("missing added file should fail");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(
        result.glyph_cache.is_some(),
        "failed regenerate must leave the incremental cache available"
    );
    assert_eq!(result.svg_string().unwrap(), before);

    write_icon(&dir, "c", D3);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(
                c.clone(),
                GlyphChange::Added {
                    name: Some("c".to_owned()),
                },
            )],
        )
        .unwrap();

    assert_same(&result, &generate(vec![a, b, c], false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_rejects_duplicate_glyph_names() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let mut result = generate(vec![a.clone(), b.clone()], true);
    let before = result.svg_string().unwrap().to_owned();

    let error = result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(
                b.clone(),
                GlyphChange::Changed {
                    name: Some("a".to_owned()),
                },
            )],
        )
        .expect_err("duplicate names should match fresh generate validation");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("must be unique"));
    assert!(result.glyph_cache.is_some());
    assert_eq!(result.svg_string().unwrap(), before);
    std::fs::remove_dir_all(&dir).ok();
}

fn generate_with_css(paths: Vec<String>, incremental: bool) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(true),
        dest: "artifacts".to_owned(),
        files: paths,
        html: Some(false),
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options()),
        ligature: Some(false),
        incremental: Some(incremental),
        // These tests assert in-memory parity; don't touch the disk on regenerate.
        write_files: Some(false),
        types: Some(vec![FontType::Woff2]),
        ..Default::default()
    })
    .unwrap();
    let source_files = load(&resolved.files);
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    generate_webfonts_sync(resolved, source_files).unwrap()
}

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
        format_options: Some(stable_format_options()),
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

fn generate_writing(paths: Vec<String>, dest: &Path) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(true),
        dest: dest.to_string_lossy().into_owned(),
        files: paths,
        html: Some(false),
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options()),
        ligature: Some(false),
        incremental: Some(true),
        write_files: Some(true),
        types: Some(vec![FontType::Woff2]),
        ..Default::default()
    })
    .unwrap();
    let source_files = load(&resolved.files);
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    generate_webfonts_sync(resolved, source_files).unwrap()
}

#[test]
fn regenerate_writes_changed_outputs_and_skips_unchanged() {
    let dir = temp_dir();
    let dest = dir.join("out");
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let files = vec![a.clone(), b.clone()];

    let mut result = generate_writing(files.clone(), &dest);
    let woff2_path = dest.join("rc.woff2");
    let css_path = dest.join("rc.css");

    // This helper builds in memory only, so this first regenerate performs the initial write.
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(&files, &[(b.clone(), GlyphChange::Changed { name: None })])
        .unwrap();

    let on_disk = std::fs::read(&woff2_path).unwrap();
    assert_eq!(on_disk.as_slice(), result.woff2_bytes().unwrap());
    let fresh = generate_with_css(files.clone(), false);
    assert_eq!(on_disk.as_slice(), fresh.woff2_bytes().unwrap());
    assert!(css_path.exists(), "CSS is written to disk too");

    // Re-running with no real change reproduces identical bytes, so the write is skipped: a
    // deleted output is NOT recreated.
    std::fs::remove_file(&woff2_path).unwrap();
    result
        .regenerate(&files, &[(b.clone(), GlyphChange::Changed { name: None })])
        .unwrap();
    assert!(
        !woff2_path.exists(),
        "an unchanged output must not be rewritten"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn initial_write_seeds_skip_map_for_first_regenerate() {
    let dir = temp_dir();
    let dest = dir.join("out");
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let files = vec![a.clone(), b.clone()];

    let mut result = crate::generate_sync(
        GenerateWebfontsOptions {
            css: Some(false),
            dest: dest.to_string_lossy().into_owned(),
            files: files.clone(),
            html: Some(false),
            font_name: Some("rc".to_owned()),
            format_options: Some(stable_format_options()),
            ligature: Some(false),
            incremental: Some(true),
            write_files: Some(true),
            types: Some(vec![FontType::Woff2]),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let woff2_path = dest.join("rc.woff2");
    assert!(woff2_path.exists(), "the initial build wrote the font");

    // No real change -> identical output -> skipped because the initial write seeded the hash.
    std::fs::remove_file(&woff2_path).unwrap();
    result
        .regenerate(&files, &[(b.clone(), GlyphChange::Changed { name: None })])
        .unwrap();
    assert!(
        !woff2_path.exists(),
        "first regenerate must skip an output unchanged since the seeded initial write"
    );

    std::fs::remove_dir_all(&dir).ok();
}
