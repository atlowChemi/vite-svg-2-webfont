use super::*;
use crate::input::GenerationOptions;

#[test]
fn escapes_strings_and_preserves_order_with_variant_deduplication() {
    let name = "quote\"'\\\n\r\t\0<&雪\u{2028}\u{2029}";
    let options = GenerationOptions {
        template_options: Some(Map::from_iter([
            ("baseSelector".into(), Value::String(name.into())),
            ("classPrefix".into(), Value::String("".into())),
        ])),
        ..Default::default()
    }
    .resolve(false)
    .unwrap();
    let text = render_ts(&options, [name, "a", name].into_iter(), true, None);
    let literal = serde_json::to_string(name).unwrap();
    assert!(text.starts_with(&format!(
        "export const iconBaseSelector = {literal} as const;\n"
    )));
    assert!(text.contains("export const iconClassPrefix = \"\" as const;"));
    assert!(text.contains(&format!(
        "export const icons = [\n  {literal},\n  \"a\"\n] as const;"
    )));
    let ordinary = render_ts(&options, ["z", "a", "z"].into_iter(), false, None);
    assert!(ordinary.contains("[\n  \"z\",\n  \"a\",\n  \"z\"\n]"));
    let empty = render_ts(&options, std::iter::empty(), false, None);
    assert!(empty.contains("export const icons = [] as const;"));
}

#[test]
fn effective_css_context_overrides_options_without_inspecting_css() {
    let options = GenerationOptions {
        template_options: Some(Map::from_iter([
            ("baseSelector".into(), ".option".into()),
            ("classPrefix".into(), "option-".into()),
        ])),
        ..Default::default()
    }
    .resolve(false)
    .unwrap();
    let context = Map::from_iter([
        ("baseSelector".into(), Value::String(".changed".into())),
        ("classPrefix".into(), Value::String("changed-".into())),
    ]);
    let text = render_ts(&options, ["add"].into_iter(), false, Some(&context));
    assert!(text.contains("\".changed\" as const"));
    assert!(text.contains("\"changed-\" as const"));
    let missing = render_ts(&options, ["add"].into_iter(), false, Some(&Map::new()));
    assert!(missing.contains("iconBaseSelector = \".icon\" as const"));
    assert!(missing.contains("iconClassPrefix = \"icon-\" as const"));
}

#[test]
fn selector_fallbacks_preserve_explicit_empty_strings() {
    for (value, base, prefix) in [
        (Value::Null, ".icon", "icon-"),
        (Value::Bool(false), ".icon", "icon-"),
        (Value::from(42), ".icon", "icon-"),
        (Value::String(String::new()), "", ""),
    ] {
        let context = Map::from_iter([
            ("baseSelector".into(), value.clone()),
            ("classPrefix".into(), value),
        ]);
        let options = GenerationOptions {
            template_options: Some(context.clone()),
            ..Default::default()
        }
        .resolve(false)
        .unwrap();
        for css_context in [None, Some(&context)] {
            let text = render_ts(&options, ["add"].into_iter(), false, css_context);
            assert!(text.contains(&format!("iconBaseSelector = {base:?} as const")));
            assert!(text.contains(&format!("iconClassPrefix = {prefix:?} as const")));
        }
    }
}

#[test]
fn borrowed_settings_match_resolved_defaults_and_explicit_overrides() {
    for overrides in [
        None,
        Some(serde_json::json!({})),
        Some(serde_json::json!({"classPrefix": "custom-"})),
        Some(serde_json::json!({"baseSelector": "", "classPrefix": null})),
        Some(serde_json::json!({"baseSelector": 42, "classPrefix": false})),
    ] {
        let options = GenerationOptions {
            template_options: overrides.map(|value| value.as_object().unwrap().clone()),
            ..Default::default()
        }
        .resolve(false)
        .unwrap();
        let resolved = crate::rendering::css::resolved_template_options(&options);
        assert_eq!(
            render_ts(&options, ["add"].into_iter(), false, None),
            render_ts(&options, ["add"].into_iter(), false, Some(&resolved)),
        );
    }
}

#[cfg(feature = "in_memory")]
mod memory {
    use crate::{
        FontType, GenerateWebfontsFromSourcesOptions, OutputKind, SourceChange, SourceFontVariant,
        SourceMembership, SvgSource, generate_from_sources_sync,
    };
    use std::sync::Arc;

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0H24V24H0Z"/></svg>"#;
    fn source(id: &str, name: &str) -> SvgSource {
        SvgSource {
            id: id.into(),
            name: name.into(),
            svg: SVG.into(),
        }
    }
    fn options() -> GenerateWebfontsFromSourcesOptions {
        GenerateWebfontsFromSourcesOptions {
            sources: vec![source("a", "add"), source("b", "remove")],
            types: Some(vec![FontType::Ttf]),
            css: Some(false),
            html: Some(false),
            ts: Some(true),
            incremental: Some(true),
            css_template: Some("{{#broken".into()),
            html_template: Some("{{#broken".into()),
            ..Default::default()
        }
    }
    #[test]
    fn collection_is_opt_in_and_ts_does_not_compile_templates() {
        let result = generate_from_sources_sync(options()).unwrap();
        let text = result.generate_ts().unwrap();
        assert!(text.contains("[\n  \"add\",\n  \"remove\"\n]"));
        assert!(Arc::ptr_eq(&text, &result.generate_ts().unwrap()));
        let outputs = result.collect_outputs().unwrap();
        assert_eq!(outputs.len(), 2);
        assert!(
            outputs
                .iter()
                .any(|o| o.kind == OutputKind::Ts && o.contents.as_bytes() == text.as_bytes())
        );
        assert!(result.generate_css().is_err());
        let result = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
            ts: None,
            ..options()
        })
        .unwrap();
        assert_eq!(result.collect_outputs().unwrap().len(), 1);
        assert_eq!(result.generate_ts().unwrap(), text);
    }
    #[test]
    fn regeneration_refreshes_membership_but_reuses_geometry_only_output() {
        let mut result = generate_from_sources_sync(options()).unwrap();
        let initial = result.generate_ts().unwrap();
        result
            .regenerate_sync(
                SourceMembership::Sources(vec!["a".into(), "b".into()]),
                vec![SourceChange::Changed {
                    id: "a".into(),
                    name: None,
                    svg: SVG.replace("24H0", "20H0"),
                }],
            )
            .unwrap();
        assert!(Arc::ptr_eq(&initial, &result.generate_ts().unwrap()));
        result
            .regenerate_sync(
                SourceMembership::Sources(vec!["b".into(), "c".into()]),
                vec![
                    SourceChange::Added(source("c", "new")),
                    SourceChange::Changed {
                        id: "b".into(),
                        name: Some("renamed".into()),
                        svg: SVG.into(),
                    },
                ],
            )
            .unwrap();
        let updated = result.generate_ts().unwrap();
        assert!(updated.contains("[\n  \"renamed\",\n  \"new\"\n]"));
        assert!(!Arc::ptr_eq(&initial, &updated));
        assert!(
            result
                .regenerate_sync(SourceMembership::Sources(vec!["missing".into()]), vec![])
                .is_err()
        );
        assert!(Arc::ptr_eq(&updated, &result.generate_ts().unwrap()));
    }
    #[test]
    fn variants_export_ordered_unique_names() {
        let result = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
            sources: vec![],
            variants: Some(vec![
                SourceFontVariant {
                    name: "regular".into(),
                    weight: Some(400),
                    default: Some(true),
                    sources: vec![source("r1", "add"), source("r2", "remove")],
                },
                SourceFontVariant {
                    name: "bold".into(),
                    weight: Some(700),
                    default: None,
                    sources: vec![source("b1", "add"), source("b2", "remove")],
                },
            ]),
            ..options()
        })
        .unwrap();
        assert!(
            result
                .generate_ts()
                .unwrap()
                .contains("[\n  \"add\",\n  \"remove\"\n]")
        );
    }
}

#[cfg(feature = "fs")]
mod files {
    use crate::test_helpers::{webfont_fixture, write_temp_template};
    use crate::{FontType, GenerateWebfontsOptions, RegenerationFiles, generate_sync};
    use std::sync::Arc;

    fn options() -> GenerateWebfontsOptions {
        GenerateWebfontsOptions {
            files: vec![webfont_fixture("add.svg")],
            dest: "unused".into(),
            types: Some(vec![FontType::Ttf]),
            css: Some(false),
            html: Some(false),
            write_files: Some(false),
            incremental: Some(true),
            ..Default::default()
        }
    }
    #[test]
    fn renders_without_reading_templates_and_matches_html_workaround() {
        let result = generate_sync(
            GenerateWebfontsOptions {
                css_template: Some("/missing/css.hbs".into()),
                html_template: Some("/missing/html.hbs".into()),
                ..options()
            },
            None,
        )
        .unwrap();
        let ts = result.generate_ts().unwrap();
        assert!(Arc::ptr_eq(&ts, &result.generate_ts().unwrap()));
        let template = write_temp_template(
            "ts-parity",
            "export const iconBaseSelector = \"{{{baseSelector}}}\" as const;\nexport const iconClassPrefix = \"{{{classPrefix}}}\" as const;\nexport const icons = [{{#each names}}{{#unless @first}},{{/unless}}\n  \"{{{this}}}\"{{/each}}\n] as const;\n\nexport type IconName = (typeof icons)[number];\n",
        );
        let legacy = generate_sync(
            GenerateWebfontsOptions {
                html_template: Some(template.clone()),
                ..options()
            },
            None,
        )
        .unwrap();
        assert_eq!(legacy.generate_html_pure(None).unwrap(), *ts);
        std::fs::remove_file(template).unwrap();
    }
    #[test]
    fn writes_default_and_custom_destinations_and_refreshes_membership() {
        let marker = write_temp_template("ts-output", "");
        std::fs::remove_file(&marker).unwrap();
        let dir = std::path::Path::new(&marker);
        let mut result = generate_sync(
            GenerateWebfontsOptions {
                dest: marker.clone(),
                ts: Some(true),
                write_files: Some(true),
                ..options()
            },
            None,
        )
        .unwrap();
        let output = dir.join("iconfont.ts");
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            *result.generate_ts().unwrap()
        );
        let extra = dir.join("extra.svg");
        std::fs::copy(webfont_fixture("add.svg"), &extra).unwrap();
        result
            .regenerate_all(&RegenerationFiles::Single(vec![
                extra.to_string_lossy().into_owned(),
            ]))
            .unwrap();
        assert!(result.generate_ts().unwrap().contains("[\n  \"extra\"\n]"));
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            *result.generate_ts().unwrap()
        );
        let custom = dir.join("nested/custom.ts");
        generate_sync(
            GenerateWebfontsOptions {
                dest: marker.clone(),
                ts: Some(true),
                ts_dest: Some(custom.to_string_lossy().into_owned()),
                write_files: Some(true),
                ..options()
            },
            None,
        )
        .unwrap();
        assert!(custom.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[tokio::test]
    async fn css_hook_settings_are_used_even_when_css_is_disabled() {
        struct Hooks;
        impl crate::GenerationHooks for Hooks {
            type Error = std::io::Error;
            fn has_css_context(&self) -> bool {
                true
            }
            async fn css_context(
                &self,
                mut ctx: serde_json::Map<String, serde_json::Value>,
            ) -> std::io::Result<serde_json::Map<String, serde_json::Value>> {
                ctx.insert("baseSelector".into(), ".hook".into());
                ctx.insert("classPrefix".into(), "hook-".into());
                Ok(ctx)
            }
        }
        let result = crate::generate_with_hooks(options(), &Hooks).await.unwrap();
        let text = result.generate_ts().unwrap();
        assert!(text.contains("\".hook\" as const"));
        assert!(text.contains("\"hook-\" as const"));
    }

    #[test]
    fn variants_refresh_types_on_rename_and_keep_them_for_geometry_changes() {
        let marker = write_temp_template("ts-variants", "");
        std::fs::remove_file(&marker).unwrap();
        std::fs::create_dir_all(&marker).unwrap();
        let path = std::path::Path::new(&marker).join("add.svg");
        std::fs::copy(webfont_fixture("add.svg"), &path).unwrap();
        let path = path.to_string_lossy().into_owned();
        let mut result = generate_sync(
            GenerateWebfontsOptions {
                files: vec![],
                variants: Some(vec![
                    crate::FontVariant {
                        name: "regular".into(),
                        files: vec![path.clone()],
                        weight: Some(400),
                        default: Some(true),
                    },
                    crate::FontVariant {
                        name: "bold".into(),
                        files: vec![path.clone()],
                        weight: Some(700),
                        default: None,
                    },
                ]),
                ..options()
            },
            None,
        )
        .unwrap();
        let before = result.generate_ts().unwrap();
        assert!(before.contains("[\n  \"add\"\n]"));
        let svg = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, svg.replace("</svg>", "<!-- changed --></svg>")).unwrap();
        result.update(path.clone(), None).unwrap();
        assert!(Arc::ptr_eq(&before, &result.generate_ts().unwrap()));
        result.update(path, Some("renamed".into())).unwrap();
        assert!(
            result
                .generate_ts()
                .unwrap()
                .contains("[\n  \"renamed\"\n]")
        );
        std::fs::remove_dir_all(marker).unwrap();
    }

    #[test]
    fn failed_write_restores_previous_types_and_snapshot_updates_are_independent() {
        let marker = write_temp_template("ts-failed-write", "");
        std::fs::remove_file(&marker).unwrap();
        let mut result = generate_sync(
            GenerateWebfontsOptions {
                dest: marker.clone(),
                ts: Some(true),
                write_files: Some(true),
                ..options()
            },
            None,
        )
        .unwrap();
        let original = result.generate_ts().unwrap();
        let output = std::path::Path::new(&marker).join("iconfont.ts");
        std::fs::remove_file(&output).unwrap();
        std::fs::create_dir(&output).unwrap();
        assert!(
            result
                .update(webfont_fixture("add.svg"), Some("renamed".into()))
                .is_err()
        );
        assert!(Arc::ptr_eq(&original, &result.generate_ts().unwrap()));
        std::fs::remove_dir_all(marker).unwrap();
    }

    #[tokio::test]
    async fn snapshot_types_do_not_change_when_replacement_is_rendered() {
        let result = crate::generate(options(), None).await.unwrap();
        let original = result.generate_ts().unwrap();
        let replacement = result
            .update_snapshot_async(webfont_fixture("add.svg"), Some("renamed".into()))
            .await
            .unwrap();
        assert!(
            replacement
                .generate_ts()
                .unwrap()
                .contains("[\n  \"renamed\"\n]")
        );
        assert!(Arc::ptr_eq(&original, &result.generate_ts().unwrap()));
    }
}
