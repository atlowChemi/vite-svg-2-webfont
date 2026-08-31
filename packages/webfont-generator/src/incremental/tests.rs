use std::collections::HashMap;
use std::path::Path;

use serde_json::{Map, Value};
use write_fonts::read::{FontRef, TableProvider};

use crate::input::{
    LoadedSvgFile, finalize_generate_webfonts_options, resolve_generate_webfonts_options,
};
use crate::pipeline::generate_webfonts_sync;
use crate::test_helpers::write_temp_template;
use crate::types::{FontType, GenerateWebfontsResult, GlyphChange, RegenerationState};
use crate::{FormatOptions, GenerateWebfontsOptions, TtfFormatOptions};

const D1: &str = "M2 2 L22 2 L22 22 Z";
const D2: &str = "M2 2 L22 2 L12 22 Z";
const D3: &str = "M4 4 L20 4 L20 20 L4 20 Z";
const D_CHANGED: &str = "M0 0 L24 0 L24 24 Z";
const TEST_TTF_TIMESTAMP: i64 = 1_700_000_000;

fn with_regeneration_state<T>(
    result: &GenerateWebfontsResult,
    read: impl FnOnce(&RegenerationState) -> T,
) -> T {
    let state = result.regeneration_state.lock().unwrap();
    read(state.as_ref().unwrap())
}

fn stable_format_options(with_metadata: bool) -> FormatOptions {
    FormatOptions {
        ttf: Some(TtfFormatOptions {
            copyright: with_metadata.then(|| "Copyright 2026".to_owned()),
            description: with_metadata.then(|| "Incremental test font".to_owned()),
            ts: Some(TEST_TTF_TIMESTAMP),
            url: with_metadata.then(|| "https://example.com".to_owned()),
            version: with_metadata.then(|| "1.0".to_owned()),
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
    write_icon_with_viewbox(dir, name, 24, 24, d)
}

fn write_icon_with_viewbox(dir: &Path, name: &str, width: u32, height: u32, d: &str) -> String {
    let path = dir.join(format!("{name}.svg"));
    std::fs::write(
        &path,
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\"><path d=\"{d}\"/></svg>"),
    )
    .unwrap();
    path.to_string_lossy().into_owned()
}

fn load(paths: &[String]) -> Vec<LoadedSvgFile> {
    paths
        .iter()
        .map(|path| LoadedSvgFile {
            contents: std::fs::read_to_string(path).unwrap().into(),
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
    generate_with_ligatures(paths, incremental, false, false)
}

fn generate_with_ligatures(
    paths: Vec<String>,
    incremental: bool,
    ligature: bool,
    with_metadata: bool,
) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: paths,
        html: Some(false),
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options(with_metadata)),
        ligature: Some(ligature),
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

#[test]
fn ligature_regeneration_invalidates_name_dependent_tables_and_skips_single_characters() {
    let dir = temp_dir();
    let ab = write_icon(&dir, "ab", D1);
    let x = write_icon(&dir, "x", "");
    let mut result = generate_with_ligatures(vec![ab.clone(), x.clone()], true, true, true);
    let before = with_regeneration_state(&result, |state| {
        state.ttf_cache.as_ref().unwrap().table_compile_count
    });

    let initial = FontRef::new(result.ttf_bytes().unwrap()).expect("readable ligature TTF");
    assert!(initial.gsub().is_ok());
    assert!(
        initial
            .cmap()
            .unwrap()
            .map_codepoint(u32::from('a'))
            .is_some()
    );
    assert!(
        initial
            .cmap()
            .unwrap()
            .map_codepoint(u32::from('x'))
            .is_none()
    );

    result
        .regenerate(
            &[ab.clone(), x.clone()],
            &[(
                ab.clone(),
                GlyphChange::Changed {
                    name: Some("cd".to_owned()),
                },
            )],
        )
        .unwrap();
    let cd = write_icon(&dir, "cd", D1);

    assert_same(
        &result,
        &generate_with_ligatures(vec![cd, x], false, true, true),
    );
    assert_eq!(
        with_regeneration_state(&result, |state| state
            .ttf_cache
            .as_ref()
            .unwrap()
            .table_compile_count),
        before + 3,
        "only cmap, post, and GSUB should be rebuilt for a same-length ligature rename"
    );
    std::fs::remove_dir_all(&dir).ok();
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

#[tokio::test]
async fn regenerate_async_matches_fresh_and_recovers_after_failure() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let files = vec![a.clone(), b.clone()];
    let result = generate(files.clone(), true);

    std::fs::remove_file(&b).unwrap();
    let error = match result
        .regenerate_async(
            files.clone(),
            vec![(b.clone(), GlyphChange::Changed { name: None })],
        )
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("missing changed file should fail"),
    };
    let result = error
        .into_result()
        .expect("ordinary failures are recoverable");

    write_icon(&dir, "b", D_CHANGED);
    let result = result
        .regenerate_async(
            files.clone(),
            vec![(b, GlyphChange::Changed { name: None })],
        )
        .await
        .unwrap();
    assert_same(&result, &generate(files, false));
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn regenerate_all_async_matches_fresh() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let files = vec![a.clone(), b.clone()];
    let result = generate(files.clone(), true);

    write_icon(&dir, "b", D_CHANGED);
    let result = result.regenerate_all_async(files.clone()).await.unwrap();

    assert_same(&result, &generate(files, false));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_reuses_compiled_ttf_glyphs_for_stable_metrics() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![a.clone(), b.clone(), c.clone()], true);
    let (processed_before, before, woff2_before) = with_regeneration_state(&result, |state| {
        let ttf = state.ttf_cache.as_ref().unwrap();
        (
            state.glyph_cache.process_count,
            ttf.compile_count,
            ttf.woff2_transform_compile_count(),
        )
    });
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();

    assert_same(&result, &generate(vec![a, b, c], false));
    assert_eq!(
        with_regeneration_state(&result, |state| state.glyph_cache.process_count),
        processed_before + 1
    );
    assert_eq!(
        with_regeneration_state(&result, |state| state
            .ttf_cache
            .as_ref()
            .unwrap()
            .compile_count),
        before + 1
    );
    assert_eq!(
        with_regeneration_state(&result, |state| state
            .ttf_cache
            .as_ref()
            .unwrap()
            .woff2_transform_compile_count()),
        woff2_before + 1
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_reuses_unchanged_ttf_tables_on_rename() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![a.clone(), b.clone(), c.clone()], true);
    let (before, woff_before, woff2_before) = with_regeneration_state(&result, |state| {
        let ttf = state.ttf_cache.as_ref().unwrap();
        (
            ttf.table_compile_count,
            ttf.woff1_payload_compile_count(),
            ttf.woff2_transform_compile_count(),
        )
    });
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(
                b.clone(),
                GlyphChange::Changed {
                    name: Some("renamed".to_owned()),
                },
            )],
        )
        .unwrap();

    assert_eq!(
        with_regeneration_state(&result, |state| state
            .ttf_cache
            .as_ref()
            .unwrap()
            .table_compile_count),
        before + 1
    );
    assert_eq!(
        with_regeneration_state(&result, |state| state
            .ttf_cache
            .as_ref()
            .unwrap()
            .woff1_payload_compile_count()),
        woff_before + 2
    );
    assert_eq!(
        with_regeneration_state(&result, |state| state
            .ttf_cache
            .as_ref()
            .unwrap()
            .woff2_transform_compile_count()),
        woff2_before
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_recompiles_compiled_ttf_glyphs_after_metric_shift() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let c = write_icon(&dir, "c", D3);

    let mut result = generate(vec![a.clone(), b.clone(), c.clone()], true);
    let (processed_before, before) = with_regeneration_state(&result, |state| {
        (
            state.glyph_cache.process_count,
            state.ttf_cache.as_ref().unwrap().compile_count,
        )
    });
    write_icon_with_viewbox(&dir, "b", 24, 48, D_CHANGED);
    result
        .regenerate(
            &[a.clone(), b.clone(), c.clone()],
            &[(b.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();

    assert_same(&result, &generate(vec![a, b, c], false));
    assert_eq!(
        with_regeneration_state(&result, |state| state.glyph_cache.process_count),
        processed_before + 3
    );
    assert_eq!(
        with_regeneration_state(&result, |state| state
            .ttf_cache
            .as_ref()
            .unwrap()
            .compile_count),
        before + 3
    );
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
    let before = with_regeneration_state(&result, |state| state.glyph_cache.parse_count);

    result.regenerate_all(&[a, b]).unwrap();

    assert_eq!(
        with_regeneration_state(&result, |state| state.glyph_cache.parse_count),
        before
    );
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
        result.regeneration_state.lock().unwrap().is_none(),
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
    with_regeneration_state(&result, |state| {
        assert_eq!(state.glyph_cache.entries.len(), 2);
        assert_eq!(state.glyph_cache.content_hashes.len(), 2);
        assert_eq!(state.glyph_cache.by_content_hash.len(), 2);
    });
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn regenerate_noop_changed_event_returns_before_parsing() {
    let dir = temp_dir();
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let mut result = generate(vec![a.clone(), b.clone()], true);
    let before = with_regeneration_state(&result, |state| state.glyph_cache.parse_count);

    result
        .regenerate(
            &[a.clone(), b.clone()],
            &[(b, GlyphChange::Changed { name: None })],
        )
        .unwrap();

    assert_eq!(
        with_regeneration_state(&result, |state| state.glyph_cache.parse_count),
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
    let before = with_regeneration_state(&result, |state| state.glyph_cache.parse_count);

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
        with_regeneration_state(&result, |state| state.glyph_cache.parse_count),
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

    with_regeneration_state(&result, |state| {
        assert_eq!(state.glyph_cache.entries.len(), 2);
        assert_eq!(state.glyph_cache.content_hashes.len(), 2);
        assert_eq!(state.glyph_cache.by_content_hash.len(), 2);
        assert!(state.glyph_cache.entries.contains_key(&a));
        assert!(state.glyph_cache.entries.contains_key(&c));
    });
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

        with_regeneration_state(&result, |state| {
            assert_eq!(state.glyph_cache.entries.len(), 3);
            assert_eq!(state.glyph_cache.content_hashes.len(), 3);
            assert_eq!(state.glyph_cache.by_content_hash.len(), 3);
        });

        result
            .regenerate(&[a.clone(), b.clone()], &[(extra, GlyphChange::Removed)])
            .unwrap();

        with_regeneration_state(&result, |state| {
            assert_eq!(state.glyph_cache.entries.len(), 2);
            assert_eq!(state.glyph_cache.content_hashes.len(), 2);
            assert_eq!(state.glyph_cache.by_content_hash.len(), 2);
            assert!(state.glyph_cache.entries.contains_key(&a));
            assert!(state.glyph_cache.entries.contains_key(&b));
        });
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
    let parse_count = with_regeneration_state(&result, |state| state.glyph_cache.parse_count);

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
        result.regeneration_state.lock().unwrap().is_some(),
        "failed regenerate must leave the incremental cache available"
    );
    assert_eq!(result.svg_string().unwrap(), before);
    assert_eq!(
        with_regeneration_state(&result, |state| state.glyph_cache.parse_count),
        parse_count,
        "failure before cache mutation should preserve the warm cache"
    );

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
    assert!(result.regeneration_state.lock().unwrap().is_some());
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
        format_options: Some(stable_format_options(false)),
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

fn generate_writing(paths: Vec<String>, dest: &Path) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(true),
        dest: dest.to_string_lossy().into_owned(),
        files: paths,
        html: Some(false),
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options(false)),
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
fn regenerate_write_failure_restores_published_output() {
    let dir = temp_dir();
    let dest = dir.join("blocked-output");
    let a = write_icon(&dir, "a", D1);
    let b = write_icon(&dir, "b", D2);
    let files = vec![a.clone(), b.clone()];
    let mut result = generate_writing(files.clone(), &dest);
    let before = result.woff2_bytes().unwrap().to_vec();

    std::fs::write(&dest, "not a directory").unwrap();
    write_icon(&dir, "b", D_CHANGED);
    result
        .regenerate(&files, &[(b.clone(), GlyphChange::Changed { name: None })])
        .expect_err("writing below a file must fail");
    assert_eq!(result.woff2_bytes().unwrap(), before);

    std::fs::remove_file(&dest).unwrap();
    result
        .regenerate(&files, &[(b, GlyphChange::Changed { name: None })])
        .unwrap();
    assert_same(&result, &generate_with_css(files, false));
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
            format_options: Some(stable_format_options(false)),
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
