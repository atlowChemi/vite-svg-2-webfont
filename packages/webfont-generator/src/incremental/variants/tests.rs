use super::*;
use crate::{FontType, FontVariant, GenerateWebfontsOptions};

#[test]
fn stable_edits_reuse_geometry_and_noops_preserve_outputs_without_cache_growth() {
    let root = std::env::temp_dir().join(format!("variant-cache-counts-{}", std::process::id()));
    let files: Vec<_> = (0..2)
        .map(|index| {
            let dir = root.join(index.to_string());
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("add.svg");
            std::fs::write(&path, svg(10 + index)).unwrap();
            path.to_string_lossy().into_owned()
        })
        .collect();
    let mut result = crate::generate_sync(
        GenerateWebfontsOptions {
            dest: root.to_string_lossy().into_owned(),
            incremental: Some(true),
            write_files: Some(false),
            types: Some(vec![FontType::Ttf]),
            variants: Some(
                files
                    .iter()
                    .enumerate()
                    .map(|(index, file)| FontVariant {
                        name: index.to_string(),
                        files: vec![file.clone()],
                        weight: None,
                        default: Some(index == 0),
                    })
                    .collect(),
            ),
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let sets = crate::RegenerationFiles::Variants(
        files
            .iter()
            .enumerate()
            .map(|(index, path)| VariantFileSet {
                variant: index.to_string(),
                files: vec![path.clone()],
            })
            .collect(),
    );
    let counts = |result: &GenerateWebfontsResult| {
        let state = result.regeneration_state.lock().unwrap();
        let cache = state.as_ref().unwrap().variant_cache.as_ref().unwrap();
        (
            cache
                .parsed
                .iter()
                .map(|cache| cache.parse_count)
                .sum::<usize>(),
            cache.process_count,
        )
    };
    assert_eq!(counts(&result), (2, 2));
    let bytes = result.fonts.ttf_font.clone().unwrap();
    result.regenerate_all(&sets).unwrap();
    assert!(Arc::ptr_eq(&bytes, result.fonts.ttf_font.as_ref().unwrap()));
    assert_eq!(counts(&result), (2, 2));
    for index in 0..20 {
        std::fs::write(&files[1], svg(12 + index)).unwrap();
        result.regenerate_all(&sets).unwrap();
        assert_eq!(counts(&result), (3 + index, 3 + index));
        let state = result.regeneration_state.lock().unwrap();
        let cache = state.as_ref().unwrap().variant_cache.as_ref().unwrap();
        assert_eq!(cache.processed.len(), 2);
        // Only the edited presentation materializes parsed paths. The unchanged design
        // reaches its processed cache without cloning its parsed geometry.
        assert_eq!(cache.materialize_count, 3 + index);
        for parsed in &cache.parsed {
            assert_eq!(parsed.entries.len(), 1);
            assert_eq!(parsed.content_hashes.len(), 1);
            assert_eq!(parsed.by_content_hash.len(), 1);
        }
    }
    #[cfg(feature = "napi")]
    {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let fresh_sets = || crate::types::RegenerationFileOptions {
            files: None,
            variants: Some(
                files
                    .iter()
                    .enumerate()
                    .map(|(index, path)| VariantFileSet {
                        variant: index.to_string(),
                        files: vec![path.clone()],
                    })
                    .collect(),
            ),
        };
        result.regenerate_from_js(fresh_sets(), None).unwrap();
        let next = runtime
            .block_on(result.regenerate_async_from_js(fresh_sets(), Some(vec![])))
            .unwrap();
        assert!(
            runtime
                .block_on(result.regenerate_async_from_js(fresh_sets(), Some(vec![])))
                .is_err()
        );
        result = runtime
            .block_on(next.regenerate_async_from_js(fresh_sets(), None))
            .unwrap();
        let previous = result.ttf_bytes().unwrap().to_vec();
        std::fs::write(&files[1], "invalid SVG").unwrap();
        assert!(
            runtime
                .block_on(result.regenerate_async_from_js(fresh_sets(), None))
                .is_err()
        );
        assert_eq!(result.ttf_bytes().unwrap(), previous);
        std::fs::write(&files[1], svg(35)).unwrap();
        result = runtime
            .block_on(result.regenerate_async_from_js(fresh_sets(), None))
            .unwrap();
        result.css_context = Some(Default::default());
        assert!(
            result
                .regenerate_from_js(fresh_sets(), Some(vec![]))
                .is_err()
        );
        assert!(
            runtime
                .block_on(result.regenerate_async_from_js(fresh_sets(), Some(vec![])))
                .is_err()
        );
        result.css_context = None;
    }

    // A parse failure discards the cache mutated during the attempt, but preserves the
    // published font and source membership. Retrying repopulates it from those sources.
    let published = result.ttf_bytes().unwrap().to_vec();
    std::fs::write(&files[1], "invalid SVG").unwrap();
    assert!(result.regenerate_all(&sets).is_err());
    assert_eq!(result.ttf_bytes().unwrap(), published);
    assert!(
        result
            .regeneration_state
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .variant_cache
            .is_none()
    );
    std::fs::write(&files[1], svg(38)).unwrap();
    result.regenerate_all(&sets).unwrap();
    assert_eq!(counts(&result), (2, 2));

    // A failed write commits the new in-memory generation; a no-op retry must finish disk output.
    Arc::make_mut(&mut result.options).write_files = true;
    let blocked = root.join("iconfont.ttf");
    std::fs::create_dir(&blocked).unwrap();
    let before = result.ttf_bytes().unwrap().to_vec();
    std::fs::write(&files[1], svg(40)).unwrap();
    assert!(result.regenerate_all(&sets).is_err());
    assert_ne!(result.ttf_bytes().unwrap(), before);
    let after = counts(&result);
    std::fs::remove_dir(&blocked).unwrap();
    result.regenerate_all(&sets).unwrap();
    assert_eq!(counts(&result), after);
    assert_eq!(
        std::fs::read(&blocked).unwrap(),
        result.ttf_bytes().unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn svg(width: usize) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M0 0H{width}V24H0Z\"/></svg>"
    )
}
