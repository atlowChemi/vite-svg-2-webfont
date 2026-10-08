use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use webfont_generator::{
    ColorGlyphSelection, FontType, FontVariant, FormatOptions, GenerateWebfontsOptions,
    GenerateWebfontsResult, GlyphChange, MissingGlyphBehavior, MissingGlyphOptions,
    RegenerationFiles, TtfFormatOptions, VariantFileSet, generate_sync,
};
use write_fonts::read::{FontRef, TableProvider};

struct Inputs {
    root: PathBuf,
    files: Vec<String>,
}

impl Inputs {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "color-api-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let files = ["icon", "other"]
            .map(|name| {
                root.join(format!("{name}.svg"))
                    .to_string_lossy()
                    .into_owned()
            })
            .to_vec();
        for (path, color) in files.iter().zip(["red", "blue"]) {
            std::fs::write(path, svg(color)).unwrap();
        }
        Self { root, files }
    }

    fn options(&self, variants: bool) -> GenerateWebfontsOptions {
        GenerateWebfontsOptions {
            dest: self.root.join("out").to_string_lossy().into_owned(),
            files: if variants { vec![] } else { self.files.clone() },
            variants: variants.then(|| {
                vec![
                    FontVariant {
                        name: "Light".into(),
                        files: self.files.clone(),
                        weight: Some(300),
                        default: Some(true),
                    },
                    FontVariant {
                        name: "Bold".into(),
                        files: vec![self.files[1].clone()],
                        weight: Some(700),
                        default: None,
                    },
                ]
            }),
            color_glyphs: Some(ColorGlyphSelection::Named(vec!["icon".into()])),
            types: Some(vec![FontType::Ttf, FontType::Woff, FontType::Woff2]),
            incremental: Some(true),
            write_files: Some(false),
            format_options: Some(FormatOptions {
                ttf: Some(TtfFormatOptions {
                    ts: Some(0),
                    copyright: None,
                    description: None,
                    url: None,
                    version: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl Drop for Inputs {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

fn svg(color: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path fill="{color}" d="M10 10H90V90H10Z"/></svg>"#
    )
}

fn membership(options: &GenerateWebfontsOptions) -> RegenerationFiles {
    match &options.variants {
        Some(variants) => RegenerationFiles::Variants(
            variants
                .iter()
                .map(|v| VariantFileSet {
                    variant: v.name.clone(),
                    files: v.files.clone(),
                })
                .collect(),
        ),
        None => RegenerationFiles::Single(options.files.clone()),
    }
}

fn snapshot(result: &GenerateWebfontsResult) -> (Vec<Vec<u8>>, String) {
    (
        [
            result.ttf_bytes(),
            result.woff_bytes(),
            result.woff2_bytes(),
        ]
        .into_iter()
        .map(|b| b.unwrap().to_vec())
        .collect(),
        result.generate_css_pure(None).unwrap(),
    )
}

#[test]
fn color_selection_changes_css_hash_for_same_sources() {
    let inputs = Inputs::new();
    for variants in [false, true] {
        let generate = |selection| {
            let mut options = inputs.options(variants);
            options.color_glyphs = selection;
            snapshot(&generate_sync(options, None).unwrap())
        };
        let mono = generate(None);
        assert_eq!(mono, generate(Some(ColorGlyphSelection::Named(vec![]))));
        let all = generate(Some(ColorGlyphSelection::All));
        let named = generate(Some(ColorGlyphSelection::Named(vec!["icon".into()])));
        assert_ne!(mono.0, all.0);
        assert_ne!(mono.1, all.1, "enabling color must change CSS font URLs");
        assert_ne!(named.0, all.0);
        assert_ne!(
            named.1, all.1,
            "changing selection must change CSS font URLs"
        );
        assert_eq!(
            generate(Some(ColorGlyphSelection::Named(vec![
                "icon".into(),
                "other".into()
            ]))),
            generate(Some(ColorGlyphSelection::Named(vec![
                "other".into(),
                "icon".into(),
                "other".into()
            ])))
        );
    }
}

#[test]
fn public_color_selection_and_format_validation() {
    let inputs = Inputs::new();
    for variants in [false, true] {
        for (selection, bases) in [
            (None, 0),
            (Some(ColorGlyphSelection::Named(vec![])), 0),
            (Some(ColorGlyphSelection::All), 2),
            (
                Some(ColorGlyphSelection::Named(vec![
                    "icon".into(),
                    "icon".into(),
                ])),
                1,
            ),
        ] {
            let mut options = inputs.options(variants);
            options.color_glyphs = selection;
            let result = generate_sync(options, None).unwrap();
            let font = FontRef::new(result.ttf_bytes().unwrap()).unwrap();
            if bases == 0 {
                assert!(font.colr().is_err());
            } else {
                assert_eq!(
                    font.colr()
                        .unwrap()
                        .base_glyph_list()
                        .unwrap()
                        .unwrap()
                        .base_glyph_paint_records()
                        .len(),
                    bases
                );
            }
        }
        let mut options = inputs.options(variants);
        options.color_glyphs = Some(ColorGlyphSelection::Named(vec![
            "z".into(),
            "a".into(),
            "z".into(),
        ]));
        // Explicit codepoints do not create logical glyphs.
        options.codepoints = Some([("z".into(), 0xe001)].into());
        let error = generate_sync(options, None).err().unwrap();
        assert_eq!(
            error.to_string(),
            "options.colorGlyphs: unknown glyph names: z, a"
        );
    }
    let mut options = inputs.options(false);
    options.types = None;
    assert!(
        generate_sync(options, None)
            .err()
            .unwrap()
            .to_string()
            .contains("options.colorGlyphs: incompatible output formats: eot")
    );
}

#[test]
fn color_regeneration_preserves_parity_and_rollback() {
    for variants in [false, true] {
        for fallback in [false, true] {
            let inputs = Inputs::new();
            let mut options = inputs.options(variants);
            if variants && fallback {
                options.missing_glyphs = Some(MissingGlyphOptions {
                    behavior: MissingGlyphBehavior::Fallback,
                    variant: Some("Light".into()),
                });
            }
            let mut result = generate_sync(options.clone(), None).unwrap();
            let initial = snapshot(&result);
            let files = membership(&options);
            std::fs::write(&inputs.files[0], svg("blue")).unwrap();
            result
                .regenerate(
                    &files,
                    &[(inputs.files[0].clone(), GlyphChange::Changed { name: None })],
                )
                .unwrap();
            assert_ne!(snapshot(&result), initial);
            assert_eq!(
                snapshot(&result),
                snapshot(&generate_sync(options.clone(), None).unwrap())
            );
            let updated = snapshot(&result);
            result.regenerate(&files, &[]).unwrap();
            assert_eq!(snapshot(&result), updated);
            // Renaming away the selected logical name must fail transactionally.
            assert!(
                result
                    .regenerate(
                        &files,
                        &[(
                            inputs.files[0].clone(),
                            GlyphChange::Changed {
                                name: Some("renamed".into())
                            }
                        )]
                    )
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("unknown glyph names: icon")
            );
            assert_eq!(snapshot(&result), updated);
            std::fs::write(&inputs.files[0], "invalid svg").unwrap();
            assert!(result.regenerate_all(&files).is_err());
            assert_eq!(snapshot(&result), updated);
            std::fs::write(&inputs.files[0], svg("green")).unwrap();
            result.regenerate_all(&files).unwrap();
            assert_eq!(
                snapshot(&result),
                snapshot(&generate_sync(options.clone(), None).unwrap())
            );
            // Membership changes clear/restore blank and fallback presentations.
            if let Some(variants) = &mut options.variants {
                variants[1].files.insert(0, inputs.files[0].clone());
            } else {
                options.files.reverse();
            }
            result.regenerate(&membership(&options), &[]).unwrap();
            assert_eq!(
                snapshot(&result),
                snapshot(&generate_sync(options.clone(), None).unwrap())
            );
            if let Some(variants) = &mut options.variants {
                variants[1].files.remove(0);
            } else {
                options.files.remove(0);
            }
            result.regenerate_all(&membership(&options)).unwrap();
            assert_eq!(
                snapshot(&result),
                snapshot(&generate_sync(options, None).unwrap())
            );
        }
    }
}

#[test]
fn named_color_can_move_to_an_unchanged_source_in_a_sparse_union() {
    let inputs = Inputs::new();
    let mut options = inputs.options(true);
    // The selected name exists only in the non-default design; default is blank.
    options.variants.as_mut().unwrap()[0].files = vec![inputs.files[1].clone()];
    options.variants.as_mut().unwrap()[1].files = inputs.files.clone();
    let mut result = generate_sync(options.clone(), None).unwrap();
    assert!(
        FontRef::new(result.ttf_bytes().unwrap())
            .unwrap()
            .colr()
            .is_ok()
    );
    options.variants.as_mut().unwrap()[1].files.remove(0);
    // Remove the old selected file and give its name to the unchanged blue source.
    result
        .regenerate(
            &membership(&options),
            &[
                (inputs.files[0].clone(), GlyphChange::Removed),
                (
                    inputs.files[1].clone(),
                    GlyphChange::Changed {
                        name: Some("icon".into()),
                    },
                ),
            ],
        )
        .unwrap();
    let fresh = generate_sync(options, Some(Box::new(|_| "icon".into()))).unwrap();
    assert_eq!(snapshot(&result), snapshot(&fresh));
    let font = FontRef::new(result.ttf_bytes().unwrap()).unwrap();
    let palette = font.cpal().unwrap();
    assert_eq!(
        palette.color_records_array().unwrap().unwrap()[0].blue(),
        255
    );
}

#[test]
fn variant_validation_error_order_is_stable() {
    let inputs = Inputs::new();
    let mut options = inputs.options(true);
    options.color_glyphs = Some(ColorGlyphSelection::Named(vec!["missing".into()]));
    options.missing_glyphs = Some(MissingGlyphOptions {
        behavior: MissingGlyphBehavior::Error,
        variant: None,
    });
    // Union/codepoints are resolved before color names, and missing states after them.
    assert_eq!(
        generate_sync(options.clone(), None)
            .err()
            .unwrap()
            .to_string(),
        "options.colorGlyphs: unknown glyph names: missing"
    );
    options.start_codepoint = Some(u32::MAX);
    assert_eq!(
        generate_sync(options, None).err().unwrap().to_string(),
        "Unable to assign another glyph codepoint: the u32 range is exhausted."
    );
}

#[test]
fn color_write_failure_preserves_each_modes_retry_contract() {
    for variants in [false, true] {
        let inputs = Inputs::new();
        let mut options = inputs.options(variants);
        options.write_files = Some(true);
        options.font_name = Some("colors".into());
        let mut result = generate_sync(options.clone(), None).unwrap();
        let before = snapshot(&result);
        let blocked = PathBuf::from(&options.dest).join("colors.woff2");
        std::fs::remove_file(&blocked).unwrap();
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(&inputs.files[0], svg("green")).unwrap();
        let files = membership(&options);
        assert!(result.regenerate_all(&files).is_err());
        options.write_files = Some(false);
        let fresh = generate_sync(options, None).unwrap();
        if variants {
            assert_eq!(snapshot(&result), snapshot(&fresh));
        } else {
            assert_eq!(snapshot(&result), before);
        }
        std::fs::remove_dir(&blocked).unwrap();
        if variants {
            result.regenerate(&files, &[]).unwrap();
        } else {
            result.regenerate_all(&files).unwrap();
        }
        assert_eq!(snapshot(&result), snapshot(&fresh));
        assert_eq!(
            std::fs::read(blocked).unwrap(),
            fresh.woff2_bytes().unwrap()
        );
    }
}

#[test]
fn named_color_follows_identity_when_sources_are_renamed() {
    for variants in [false, true] {
        let inputs = Inputs::new();
        let options = inputs.options(variants);
        let mut result = generate_sync(options.clone(), None).unwrap();
        let files = membership(&options);
        // Swap logical identities without changing SVG bytes. Selection must
        // move to the blue source rather than follow the original red file.
        result
            .regenerate(
                &files,
                &[
                    (
                        inputs.files[0].clone(),
                        GlyphChange::Changed {
                            name: Some("other".into()),
                        },
                    ),
                    (
                        inputs.files[1].clone(),
                        GlyphChange::Changed {
                            name: Some("icon".into()),
                        },
                    ),
                ],
            )
            .unwrap();
        let fresh = generate_sync(
            options,
            Some(Box::new(|path| {
                if path.ends_with("/icon.svg") || path.ends_with("\\icon.svg") {
                    "other".into()
                } else {
                    "icon".into()
                }
            })),
        )
        .unwrap();
        assert_eq!(snapshot(&result), snapshot(&fresh));
        let font = FontRef::new(result.ttf_bytes().unwrap()).unwrap();
        let palette = font.cpal().unwrap();
        let colors = palette.color_records_array().unwrap().unwrap();
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0].blue(), 255);
        assert_eq!(colors[0].red(), 0);
    }
}

#[cfg(feature = "cli")]
#[test]
fn manifest_color_selection_shape() {
    let omitted: GenerateWebfontsOptions = serde_json::from_str(r#"{"dest":"unused"}"#).unwrap();
    assert_eq!(omitted.color_glyphs, None);
    for (json, expected) in [
        ("true", ColorGlyphSelection::All),
        ("[]", ColorGlyphSelection::Named(vec![])),
        (
            r#"["icon"]"#,
            ColorGlyphSelection::Named(vec!["icon".into()]),
        ),
    ] {
        let options: GenerateWebfontsOptions =
            serde_json::from_str(&format!(r#"{{"dest":"unused","colorGlyphs":{json}}}"#)).unwrap();
        assert_eq!(options.color_glyphs, Some(expected));
    }
    for json in ["null", "false", r#""icon""#, r#"["icon",1]"#, "{}"] {
        assert!(
            serde_json::from_str::<GenerateWebfontsOptions>(&format!(
                r#"{{"dest":"unused","colorGlyphs":{json}}}"#
            ))
            .is_err()
        );
    }
}
