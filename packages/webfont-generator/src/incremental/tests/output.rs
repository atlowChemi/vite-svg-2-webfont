use super::*;

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
