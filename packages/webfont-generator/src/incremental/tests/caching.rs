use super::*;

use write_fonts::read::{FontRef, TableProvider};

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
            ttf.woff1_payloads.compile_count(),
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
            .woff1_payloads
            .compile_count()),
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
