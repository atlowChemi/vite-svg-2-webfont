#![cfg(not(feature = "napi"))]

use std::path::PathBuf;
use webfont_generator::{
    FontType, FontVariant, FormatOptions, GenerateWebfontsOptions, GenerateWebfontsResult,
    GlyphChange, RegenerationFiles, TtfFormatOptions, VariantFileSet, generate_sync,
};

struct Family {
    root: PathBuf,
    files: [Vec<String>; 2],
}

impl Family {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "variant-incremental-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let files = std::array::from_fn(|index| {
            let dir = root.join(index.to_string());
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("icon-add.svg");
            std::fs::write(&path, svg(10 + index)).unwrap();
            vec![path.to_string_lossy().into_owned()]
        });
        Self { root, files }
    }

    fn generate(&self, incremental: bool) -> GenerateWebfontsResult {
        self.generate_with_policy(incremental, webfont_generator::MissingGlyphBehavior::Blank)
    }

    fn generate_with_policy(
        &self,
        incremental: bool,
        policy: webfont_generator::MissingGlyphBehavior,
    ) -> GenerateWebfontsResult {
        generate_sync(
            GenerateWebfontsOptions {
                missing_glyphs: Some(webfont_generator::MissingGlyphOptions {
                    variant: matches!(policy, webfont_generator::MissingGlyphBehavior::Fallback)
                        .then(|| "design-0".into()),
                    behavior: policy,
                }),
                dest: self.root.join("output").to_string_lossy().into_owned(),
                font_name: Some("incremental-family".into()),
                incremental: Some(incremental),
                write_files: Some(false),
                types: Some(vec![FontType::Ttf, FontType::Woff, FontType::Woff2]),
                variants: Some(
                    self.files
                        .iter()
                        .enumerate()
                        .map(|(index, files)| FontVariant {
                            name: format!("design-{index}"),
                            files: files.clone(),
                            weight: Some(if index == 0 { 300 } else { 700 }),
                            default: Some(index == 0),
                        })
                        .collect(),
                ),
                format_options: Some(FormatOptions {
                    ttf: Some(TtfFormatOptions {
                        ts: Some(1_700_000_000),
                        copyright: None,
                        description: None,
                        url: None,
                        version: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            None,
        )
        .unwrap()
    }

    fn update(&self, variant: usize) -> (String, GlyphChange) {
        (
            self.files[variant][0].clone(),
            GlyphChange::Changed { name: None },
        )
    }

    fn file_sets(&self) -> RegenerationFiles {
        RegenerationFiles::Variants(
            self.files
                .iter()
                .enumerate()
                .map(|(index, files)| VariantFileSet {
                    variant: format!("design-{index}"),
                    files: files.clone(),
                })
                .collect(),
        )
    }
}

impl Drop for Family {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

fn svg(width: usize) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M0 0H{width}V24H0Z\"/></svg>"
    )
}

fn bytes(result: &GenerateWebfontsResult) -> Vec<Vec<u8>> {
    [
        result.ttf_bytes(),
        result.woff_bytes(),
        result.woff2_bytes(),
    ]
    .into_iter()
    .map(|bytes| bytes.unwrap().to_vec())
    .collect()
}

#[test]
fn partial_edit_and_full_rediff_match_fresh_family() {
    let family = Family::new();
    let mut result = family.generate(true);
    std::fs::write(&family.files[1][0], svg(17)).unwrap();
    result
        .regenerate(&family.file_sets(), &[family.update(1)])
        .unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
    std::fs::write(&family.files[0][0], svg(21)).unwrap();
    result.regenerate_all(&family.file_sets()).unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
}

#[test]
fn failed_update_preserves_outputs_and_can_be_retried() {
    let family = Family::new();
    let mut result = family.generate(true);
    let original = bytes(&result);
    std::fs::write(&family.files[1][0], "not svg").unwrap();
    assert!(
        result
            .regenerate(&family.file_sets(), &[family.update(1)])
            .is_err()
    );
    assert_eq!(bytes(&result), original);
    std::fs::write(&family.files[1][0], svg(19)).unwrap();
    result
        .regenerate(&family.file_sets(), &[family.update(1)])
        .unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
}

#[test]
fn invalid_batches_are_rejected_without_changing_outputs() {
    let family = Family::new();
    let mut result = family.generate(true);
    let original = bytes(&result);
    assert!(
        result
            .regenerate(&family.file_sets(), &[family.update(0), family.update(0)])
            .is_err()
    );
    for kind in [GlyphChange::Added { name: None }, GlyphChange::Removed] {
        assert!(
            result
                .regenerate(&family.file_sets(), &[(family.files[0][0].clone(), kind)])
                .is_err()
        );
    }
    for case in 0..5 {
        let RegenerationFiles::Variants(mut sets) = family.file_sets() else {
            unreachable!()
        };
        match case {
            0 => sets[0].variant = "unknown".into(),
            1 => sets[0].variant = "design-1".into(),
            2 => sets[0].files.clear(),
            3 => sets[0].files.push(family.files[0][0].clone()),
            _ => {
                sets.pop();
            }
        }
        assert!(
            result
                .regenerate_all(&RegenerationFiles::Variants(sets))
                .is_err()
        );
    }
    assert!(
        result
            .regenerate(
                &family.file_sets(),
                &[("absent.svg".into(), GlyphChange::Changed { name: None })]
            )
            .is_err()
    );
    assert!(
        family
            .generate(false)
            .regenerate_all(&family.file_sets())
            .is_err()
    );
    assert_eq!(bytes(&result), original);
    result.regenerate_all(&family.file_sets()).unwrap();
}

#[test]
fn union_add_remove_and_rename_refresh_font_and_template_outputs() {
    let mut family = Family::new();
    let mut result = family.generate(true);
    let old_css = result.generate_css_pure(None).unwrap();
    let extra = family
        .root
        .join("0/another.svg")
        .to_string_lossy()
        .into_owned();
    std::fs::write(&extra, svg(15)).unwrap();
    family.files[0].insert(0, extra.clone());
    result.regenerate_all(&family.file_sets()).unwrap();
    let fresh = family.generate(false);
    assert_eq!(bytes(&result), bytes(&fresh));
    assert_eq!(
        result.generate_css_pure(None).unwrap(),
        fresh.generate_css_pure(None).unwrap()
    );
    assert_ne!(result.generate_css_pure(None).unwrap(), old_css);
    assert_eq!(
        result.generate_html_pure(None).unwrap(),
        fresh.generate_html_pure(None).unwrap()
    );
    let renamed = family
        .root
        .join("0/renamed.svg")
        .to_string_lossy()
        .into_owned();
    std::fs::rename(&extra, &renamed).unwrap();
    family.files[0][0] = renamed;
    result.regenerate_all(&family.file_sets()).unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
    family.files[0].remove(0);
    result.regenerate_all(&family.file_sets()).unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
}

#[test]
fn fallback_source_and_global_metric_changes_match_fresh_family() {
    let mut family = Family::new();
    let extra = family
        .root
        .join("0/extra.svg")
        .to_string_lossy()
        .into_owned();
    std::fs::write(&extra, svg(15)).unwrap();
    family.files[0].push(extra.clone());
    let mut result =
        family.generate_with_policy(true, webfont_generator::MissingGlyphBehavior::Fallback);
    std::fs::write(&extra, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 48\"><path d=\"M0 0H24V48H0Z\"/></svg>").unwrap();
    result.regenerate_all(&family.file_sets()).unwrap();
    assert_eq!(
        bytes(&result),
        bytes(
            &family.generate_with_policy(false, webfont_generator::MissingGlyphBehavior::Fallback)
        )
    );
}

#[test]
fn missing_coverage_failure_and_explicit_rename_are_retryable() {
    let mut family = Family::new();
    let mut result =
        family.generate_with_policy(true, webfont_generator::MissingGlyphBehavior::Error);
    let previous = bytes(&result);
    let extra = family
        .root
        .join("1/extra.svg")
        .to_string_lossy()
        .into_owned();
    std::fs::write(&extra, svg(13)).unwrap();
    family.files[1].push(extra);
    assert!(result.regenerate_all(&family.file_sets()).is_err());
    assert_eq!(bytes(&result), previous);
    let matching = family
        .root
        .join("0/extra.svg")
        .to_string_lossy()
        .into_owned();
    std::fs::write(&matching, svg(13)).unwrap();
    family.files[0].push(matching);
    result.regenerate_all(&family.file_sets()).unwrap();
    assert_eq!(
        bytes(&result),
        bytes(&family.generate_with_policy(false, webfont_generator::MissingGlyphBehavior::Error))
    );
    let mut changes = [family.update(0), family.update(1)];
    for change in &mut changes {
        change.1 = GlyphChange::Changed {
            name: Some("renamed".into()),
        };
    }
    result.regenerate(&family.file_sets(), &changes).unwrap();
    for index in 0..2 {
        let renamed = family
            .root
            .join(format!("{index}/renamed.svg"))
            .to_string_lossy()
            .into_owned();
        std::fs::rename(&family.files[index][0], &renamed).unwrap();
        family.files[index][0] = renamed;
    }
    assert_eq!(
        bytes(&result),
        bytes(&family.generate_with_policy(false, webfont_generator::MissingGlyphBehavior::Error))
    );
}

#[test]
fn async_updates_return_fresh_bytes_and_recover_after_failure() {
    let family = Family::new();
    let result = family.generate(true);
    std::fs::write(&family.files[1][0], "not svg").unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = match runtime
        .block_on(result.regenerate_async(family.file_sets(), vec![family.update(1)]))
    {
        Ok(_) => panic!("invalid SVG must fail"),
        Err(error) => error
            .into_result()
            .expect("failed update remains retryable"),
    };
    std::fs::write(&family.files[1][0], svg(19)).unwrap();
    let result = runtime
        .block_on(result.regenerate_all_async(family.file_sets()))
        .unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
}

#[test]
fn shared_path_changes_membership_moves_and_independent_order_match_fresh() {
    let mut family = Family::new();
    let shared = family
        .root
        .join("shared.svg")
        .to_string_lossy()
        .into_owned();
    std::fs::write(&shared, svg(8)).unwrap();
    family.files[0].insert(0, shared.clone());
    family.files[1].push(shared.clone());
    let mut result = family.generate(true);
    std::fs::write(&shared, svg(19)).unwrap();
    result
        .regenerate(
            &family.file_sets(),
            &[(shared.clone(), GlyphChange::Changed { name: None })],
        )
        .unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));

    // Removing just one membership is a file-list change, not a global Removed hint.
    family.files[0].remove(0);
    result.regenerate(&family.file_sets(), &[]).unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
    family.files[0].push(shared.clone());
    family.files[1].pop();
    result.regenerate(&family.file_sets(), &[]).unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));

    // A new family path may be assigned to multiple designs with one Added hint.
    let added = family.root.join("new.svg").to_string_lossy().into_owned();
    std::fs::write(&added, svg(16)).unwrap();
    family.files[0].insert(0, added.clone());
    family.files[1].push(added.clone());
    result
        .regenerate(
            &family.file_sets(),
            &[(added.clone(), GlyphChange::Added { name: None })],
        )
        .unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
    for paths in &mut family.files {
        paths.retain(|path| path != &added);
    }
    result
        .regenerate(&family.file_sets(), &[(added, GlyphChange::Removed)])
        .unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));

    // All variants are named, so the order of the batch itself is irrelevant.
    let RegenerationFiles::Variants(mut sets) = family.file_sets() else {
        unreachable!()
    };
    sets.reverse();
    result
        .regenerate_all(&RegenerationFiles::Variants(sets))
        .unwrap();
    assert_eq!(bytes(&result), bytes(&family.generate(false)));
}
