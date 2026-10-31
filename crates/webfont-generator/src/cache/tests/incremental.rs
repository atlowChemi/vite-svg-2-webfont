use super::*;

#[test]
fn automatic_persistence_tracks_membership_and_typescript_output() {
    let project = Project::new("automatic-ts");
    let ts_path = project.root.join("custom/icons.ts");
    let mut options = project.options(Some(&project.cache()));
    options.ts = Some(true);
    options.ts_dest = Some(ts_path.to_string_lossy().into_owned());
    options.write_files = Some(true);
    options.template_options = Some(
        serde_json::from_value(serde_json::json!({
            "baseSelector": ".glyph", "classPrefix": "glyph-"
        }))
        .unwrap(),
    );
    let mut result = run(options.clone());
    let original = result.generate_ts().unwrap();
    fs::remove_file(&ts_path).unwrap();
    let warm = run(options.clone());
    assert_eq!(work(&warm), (0, 0));
    assert_eq!(fs::read_to_string(&ts_path).unwrap(), *original);

    let edited = options.files[0].clone();
    fs::write(&edited, svg(12)).unwrap();
    result.update(edited, None).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &original,
        &result.generate_ts().unwrap()
    ));
    let added = project
        .root
        .join("icons/new.svg")
        .to_string_lossy()
        .into_owned();
    fs::write(&added, svg(16)).unwrap();
    result.add(added.clone(), None, None).unwrap();
    let renamed = project
        .root
        .join("icons/renamed.svg")
        .to_string_lossy()
        .into_owned();
    fs::rename(&added, &renamed).unwrap();
    result
        .regenerate(
            &RegenerationFiles::Single(project.icons()),
            &[
                (added, GlyphChange::Removed),
                (renamed, GlyphChange::Added { name: None }),
            ],
        )
        .unwrap();
    let removed = options.files[1].clone();
    fs::remove_file(&removed).unwrap();
    result.remove(removed).unwrap();
    result.flush_cache().unwrap();
    options.files = project.icons();
    let saved = fs::read(project.cache().join(&cache_files(&project.cache())[0])).unwrap();
    let restart = run(options.clone());
    assert_eq!(work(&restart), (0, 0));
    assert_eq!(fonts(&restart), fonts(&result));
    assert_eq!(
        restart.generate_ts().unwrap(),
        result.generate_ts().unwrap()
    );
    assert_eq!(
        fs::read_to_string(&ts_path).unwrap(),
        *result.generate_ts().unwrap()
    );
    options.cache_location = None;
    let uncached = run(options);
    assert_eq!(fonts(&restart), fonts(&uncached));
    assert_eq!(
        restart.generate_ts().unwrap(),
        uncached.generate_ts().unwrap()
    );
    result
        .regenerate_all(&RegenerationFiles::Single(project.icons()))
        .unwrap();
    result.flush_cache().unwrap();
    assert_eq!(
        saved,
        fs::read(project.cache().join(&cache_files(&project.cache())[0])).unwrap()
    );
}

#[test]
fn failed_rebuild_does_not_replace_a_pending_good_cache() {
    let project = Project::new("automatic-failure");
    let mut result = run(project.options(Some(&project.cache())));
    let path = project.icons()[0].clone();
    fs::write(&path, svg(12)).unwrap();
    result.update(path.clone(), None).unwrap();
    let expected = fonts(&result);
    fs::write(&path, "<svg broken").unwrap();
    assert!(result.update(path.clone(), None).is_err());
    result.flush_cache().unwrap();
    fs::write(&path, svg(12)).unwrap();
    let warm = run(project.options(Some(&project.cache())));
    assert_eq!(work(&warm), (0, 0));
    assert_eq!(fonts(&warm), expected);
    result.update(path, None).unwrap();
    result.flush_cache().unwrap();
    assert_eq!(fonts(&result), expected);
}

#[test]
fn failed_writes_are_retryable_including_initial_generation() {
    let project = Project::new("automatic-retry");
    fs::write(project.cache(), "not a directory").unwrap();
    let mut result = run(project.options(Some(&project.cache())));
    assert!(result.flush_cache().is_err());
    fs::remove_file(project.cache()).unwrap();
    result.flush_cache().unwrap();
    assert_eq!(work(&run(project.options(Some(&project.cache())))), (0, 0));

    let file = project.cache().join(&cache_files(&project.cache())[0]);
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    let path = project.icons()[0].clone();
    fs::write(&path, svg(11)).unwrap();
    result.update(path, None).unwrap();
    assert!(result.flush_cache().is_err());
    fs::remove_dir(&file).unwrap();
    result.flush_cache().unwrap();
    let warm = run(project.options(Some(&project.cache())));
    assert_eq!(work(&warm), (0, 0));
    assert_eq!(fonts(&warm), fonts(&result));
    assert!(fs::read_dir(project.cache()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
    run(project.options(None)).flush_cache().unwrap();
    let mut options = project.options(Some(&project.cache()));
    options.incremental = Some(false);
    run(options).flush_cache().unwrap();
}

#[tokio::test]
async fn snapshot_updates_persist_only_the_new_generation_and_old_results_can_flush() {
    let project = Project::new("automatic-snapshot");
    let original = crate::generate(project.options(Some(&project.cache())), None)
        .await
        .unwrap();
    let path = project.icons()[0].clone();
    fs::write(&path, svg(11)).unwrap();
    let next = original
        .update_snapshot_async(path.clone(), None)
        .await
        .unwrap();
    fs::write(&path, svg(12)).unwrap();
    let next = next.update_async(path, None).await.unwrap();
    original.flush_cache_async().await.unwrap();
    let warm = crate::generate(project.options(Some(&project.cache())), None)
        .await
        .unwrap();
    assert_eq!(work(&warm), (0, 0));
    assert_eq!(fonts(&warm), fonts(&next));
    assert_ne!(fonts(&original), fonts(&next));
}

#[tokio::test]
async fn variant_snapshot_write_failure_does_not_publish_but_sync_commit_does() {
    let project = Project::family("automatic-variant-failure");
    let mut options = project.variant_options(Some(&project.cache()));
    options.write_files = Some(true);
    options.ts = Some(true);
    let mut original = crate::generate(options.clone(), None).await.unwrap();
    let cache = project.cache().join(&cache_files(&project.cache())[0]);
    let before = fs::read(&cache).unwrap();
    let output = project.root.join("out/iconfont.ttf");
    fs::remove_file(&output).unwrap();
    fs::create_dir(&output).unwrap();
    let path = project.design(VARIANTS[0])[0].clone();
    fs::write(&path, svg(13)).unwrap();
    assert!(
        original
            .update_snapshot_async(path.clone(), None)
            .await
            .is_err()
    );
    original.flush_cache_async().await.unwrap();
    assert_eq!(fs::read(&cache).unwrap(), before);
    assert!(original.update(path, None).is_err());
    original.flush_cache_async().await.unwrap();
    options.write_files = Some(false);
    let warm = crate::generate(options, None).await.unwrap();
    assert_eq!(variant_work(&warm), (0, 0));
    assert_eq!(variant_fonts(&warm), variant_fonts(&original));
}

#[test]
fn variant_color_updates_persist_and_typescript_deduplicates_names() {
    let project = Project::family("automatic-variant-ts");
    let mut options = project.variant_options(Some(&project.cache()));
    options.ts = Some(true);
    options.write_files = Some(true);
    options.color_glyphs = Some(ColorGlyphSelection::All);
    let mut result = run(options.clone());
    let path = project.design(VARIANTS[0])[0].clone();
    fs::write(&path, r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#ff0000" d="M2 2H10V20H2Z"/><path fill="#0000ff" d="M12 2H20V20H12Z"/></svg>"##).unwrap();
    result.update(path, None).unwrap();
    result.flush_cache().unwrap();
    let warm = run(options.clone());
    assert_eq!(variant_work(&warm), (0, 0));
    assert_eq!(variant_fonts(&warm), variant_fonts(&result));
    assert_eq!(warm.generate_ts().unwrap().matches("\"add\"").count(), 1);
    for action in ["add", "remove", "rename"] {
        for variant in options.variants.as_mut().unwrap() {
            let directory = project.root.join(&variant.name);
            match action {
                "add" => {
                    let path = directory.join("new.svg");
                    fs::write(&path, svg(15)).unwrap();
                    variant.files.push(path.to_string_lossy().into_owned());
                }
                "remove" => {
                    let path = directory.join("remove.svg").to_string_lossy().into_owned();
                    fs::remove_file(&path).unwrap();
                    variant.files.retain(|file| file != &path);
                }
                _ => {
                    let old = directory.join("star.svg").to_string_lossy().into_owned();
                    let new = directory.join("renamed.svg").to_string_lossy().into_owned();
                    fs::rename(&old, &new).unwrap();
                    *variant.files.iter_mut().find(|file| **file == old).unwrap() = new;
                }
            }
        }
        let sets = options
            .variants
            .as_ref()
            .unwrap()
            .iter()
            .map(|variant| VariantFileSet {
                variant: variant.name.clone(),
                files: variant.files.clone(),
            })
            .collect();
        result
            .regenerate_all(&RegenerationFiles::Variants(sets))
            .unwrap();
        result.flush_cache().unwrap();
        let warm = run(options.clone());
        assert_eq!(variant_work(&warm), (0, 0));
        assert_eq!(variant_fonts(&warm), variant_fonts(&result));
        assert_eq!(warm.generate_ts().unwrap(), result.generate_ts().unwrap());
    }
    options.cache_location = None;
    let uncached = run(options);
    assert_eq!(variant_fonts(&result), variant_fonts(&uncached));
    assert_eq!(
        result.generate_ts().unwrap(),
        uncached.generate_ts().unwrap()
    );
}
