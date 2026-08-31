use super::*;

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
