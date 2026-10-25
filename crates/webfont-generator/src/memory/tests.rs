use std::collections::HashMap;
use std::sync::{Arc, atomic::Ordering};

use super::*;
#[cfg(feature = "fs")]
use crate::GenerateWebfontsOptions;
use crate::{FontType, SourceChange, SourceMembership, VariantSourceSet};

const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2H20V20H2Z"/></svg>"#;
const OTHER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0L24 24H0Z"/></svg>"#;

#[test]
fn serialized_memory_inputs_validate_and_regenerate_without_file_options() {
    let input = serde_json::json!({
        "sources": [{ "id": "opaque", "name": "icon", "svg": SVG }],
        "incremental": true,
        "types": ["ttf"],
        "formatOptions": { "ttf": { "ts": 0 } },
        "cssTemplate": "{{fontName}}",
        "fontName": "serialized"
    });
    let mut result =
        generate_from_sources_sync(serde_json::from_value(input.clone()).unwrap()).unwrap();
    assert_eq!(result.generate_css().unwrap().as_str(), "serialized");
    let before = result.ttf_bytes().unwrap().to_vec();
    let membership: SourceMembership =
        serde_json::from_value(serde_json::json!({"sources": ["opaque"]})).unwrap();
    let changes: Vec<SourceChange> = serde_json::from_value(serde_json::json!([
        { "type": "changed", "id": "opaque", "svg": OTHER }
    ]))
    .unwrap();
    result.regenerate_sync(membership, changes).unwrap();
    assert_ne!(result.ttf_bytes().unwrap(), before);
    for (key, value) in [
        ("files", serde_json::json!(["icon.svg"])),
        ("colorGlyphs", serde_json::Value::Null),
    ] {
        let mut invalid = input.clone();
        invalid[key] = value;
        assert!(serde_json::from_value::<GenerateWebfontsFromSourcesOptions>(invalid).is_err());
    }
    let mut invalid = input;
    invalid["formatOptions"]["ttf"]["unknown"] = true.into();
    assert!(serde_json::from_value::<GenerateWebfontsFromSourcesOptions>(invalid).is_err());
}

fn source(id: &str, name: &str, svg: &str) -> SvgSource {
    SvgSource {
        id: id.into(),
        name: name.into(),
        svg: svg.into(),
    }
}

fn options() -> GenerateWebfontsFromSourcesOptions {
    GenerateWebfontsFromSourcesOptions {
        sources: vec![source("opaque://not/a/file.svg", "square", SVG)],
        types: Some(vec![
            FontType::Svg,
            FontType::Ttf,
            FontType::Woff,
            FontType::Woff2,
            FontType::Eot,
        ]),
        incremental: Some(true),
        format_options: Some(crate::FormatOptions {
            ttf: Some(crate::TtfFormatOptions {
                copyright: None,
                description: None,
                ts: Some(1_700_000_000),
                url: None,
                version: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn membership(ids: &[&str]) -> SourceMembership {
    SourceMembership::Sources(ids.iter().map(|id| (*id).into()).collect())
}

#[cfg(feature = "async")]
#[tokio::test]
async fn single_source_helpers_preserve_order_and_recover_consumed_results() {
    let mut input = options();
    input.types = Some(vec![FontType::Ttf]);
    let original_id = input.sources[0].id.clone();
    let result = generate_from_sources_sync(input.clone()).unwrap();
    let result = result
        .add(source("second", "second", OTHER), None)
        .await
        .unwrap();
    input.sources.push(source("second", "second", OTHER));
    let expected = generate_from_sources_sync(input.clone()).unwrap();
    assert_fonts_equal(&result, &expected);
    let (error, result) = result
        .add(source("second", "second", SVG), None)
        .await
        .err()
        .unwrap()
        .into_parts();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "Source ID/path already exists");
    let result = result.unwrap();
    assert_fonts_equal(&result, &expected);
    let (error, result) = result
        .update("missing".into(), SVG.into(), None)
        .await
        .err()
        .unwrap()
        .into_parts();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "Unknown source ID/path");
    let result = result.unwrap();
    assert_fonts_equal(&result, &expected);
    let (error, result) = result
        .remove("missing".into())
        .await
        .err()
        .unwrap()
        .into_parts();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "Unknown source ID/path");
    let result = result.unwrap();
    assert_fonts_equal(&result, &expected);
    let (error, result) = result
        .update("second".into(), "<broken".into(), None)
        .await
        .err()
        .unwrap()
        .into_parts();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(
        error.to_string(),
        "SVG source \"second\": Failed to inspect SVG root element: the document does not have a root node"
    );
    let result = result.unwrap();
    assert_fonts_equal(&result, &expected);
    let result = result
        .update("second".into(), SVG.into(), Some("renamed".into()))
        .await
        .unwrap();
    input.sources[1] = source("second", "renamed", SVG);
    assert_fonts_equal(&result, &generate_from_sources_sync(input.clone()).unwrap());
    let mut result = result.remove(original_id).await.unwrap();
    input.sources.remove(0);
    let expected = generate_from_sources_sync(input.clone()).unwrap();
    assert_fonts_equal(&result, &expected);
    let error = result.remove_sync("second".into()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Each design must contain at least one source"
    );
    assert_fonts_equal(&result, &expected);
    result
        .add_sync(source("third", "third", OTHER), None)
        .unwrap();
    input.sources.push(source("third", "third", OTHER));
    assert_fonts_equal(&result, &generate_from_sources_sync(input.clone()).unwrap());
    result
        .update_sync("third".into(), SVG.into(), None)
        .unwrap();
    input.sources[1].svg = SVG.into();
    assert_fonts_equal(&result, &generate_from_sources_sync(input.clone()).unwrap());
    result.remove_sync("third".into()).unwrap();
    input.sources.pop();
    assert_fonts_equal(&result, &generate_from_sources_sync(input).unwrap());
}

#[test]
fn thread_counts_generate_and_update_the_same_fonts() {
    let mut input = options();
    input.sources.push(source("second", "second", OTHER));
    let mut unset = generate_from_sources_sync(input.clone()).unwrap();
    input.threads = std::num::NonZeroUsize::new(2);
    let mut two = generate_from_sources_sync(input).unwrap();
    assert_fonts_equal(&two, &unset);
    for result in [&mut unset, &mut two] {
        result
            .update_sync("second".into(), SVG.into(), None)
            .unwrap();
    }
    assert_fonts_equal(&two, &unset);
}

fn assert_fonts_equal(
    left: &GenerateWebfontsFromSourcesResult,
    right: &GenerateWebfontsFromSourcesResult,
) {
    assert_eq!(left.svg_string(), right.svg_string());
    assert_eq!(left.ttf_bytes(), right.ttf_bytes());
    assert_eq!(left.woff_bytes(), right.woff_bytes());
    assert_eq!(left.woff2_bytes(), right.woff2_bytes());
    assert_eq!(left.eot_bytes(), right.eot_bytes());
}

#[test]
#[cfg(feature = "fs")]
fn text_generation_matches_files_and_companions_stay_lazy_and_shared() {
    let path = crate::test_helpers::webfont_fixture("add.svg");
    let svg = std::fs::read_to_string(&path).unwrap();
    let file = crate::generate_sync(
        GenerateWebfontsOptions {
            dest: "unused-output".into(),
            files: vec![path],
            types: options().types,
            format_options: options().format_options,
            write_files: Some(false),
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let mut settings = options();
    settings.sources = vec![source("opaque-id", "add", &svg)];
    settings.html = Some(true);
    let memory = generate_from_sources_sync(settings).unwrap();
    assert_eq!(file.svg_string(), memory.svg_string());
    assert_eq!(file.ttf_bytes(), memory.ttf_bytes());
    assert_eq!(file.woff_bytes(), memory.woff_bytes());
    assert_eq!(file.woff2_bytes(), memory.woff2_bytes());
    assert_eq!(file.eot_bytes(), memory.eot_bytes());
    assert!(memory.shared.get().is_none());
    let css = memory.generate_css().unwrap();
    assert!(memory.html.get().is_none());
    let html = memory.generate_html().unwrap();
    assert!(html.contains(css.as_str()));
    let file_token = format!("?{}", file.get_cached_io().unwrap().shared.hash);
    let memory_token = format!("?{}", memory.shared().hash);
    assert_eq!(
        file.generate_css_pure(None)
            .unwrap()
            .replace(&file_token, "?HASH"),
        css.replace(&memory_token, "?HASH")
    );
    assert_eq!(
        file.generate_html_pure(None)
            .unwrap()
            .replace(&file_token, "?HASH"),
        html.replace(&memory_token, "?HASH")
    );
    let outputs = memory.collect_outputs().unwrap();
    assert_eq!(outputs.len(), 7);
    assert!(Arc::ptr_eq(&css, &memory.generate_css().unwrap()));
    assert!(Arc::ptr_eq(&html, &memory.generate_html().unwrap()));
    assert_eq!(memory.shared().css_render_count.load(Ordering::Relaxed), 1);
    assert_eq!(memory.html_render_count.load(Ordering::Relaxed), 1);
    let crate::OutputContents::Bytes(ttf) = &outputs[1].contents else {
        panic!("expected TTF");
    };
    assert!(Arc::ptr_eq(ttf, memory.fonts.ttf_font.as_ref().unwrap()));
}

#[test]
fn templates_are_text_and_css_does_not_compile_html() {
    let mut settings = options();
    settings.css_template = Some("{{fontName}} {{src}}".into());
    settings.html_template = Some("{{#if".into());
    let result = generate_from_sources_sync(settings.clone()).unwrap();
    result.collect_outputs().unwrap();
    assert!(result.generate_html().is_err());
    assert_eq!(result.shared().css_compile_count.load(Ordering::Relaxed), 1);
    settings.css_template = Some(String::new());
    settings.html_template = Some(String::new());
    let result = generate_from_sources_sync(settings.clone()).unwrap();
    assert_eq!(result.generate_css().unwrap().as_str(), "");
    assert_eq!(result.generate_html().unwrap().as_str(), "");
    settings.css = Some(false);
    settings.html = Some(false);
    settings.css_template = Some("{{#if".into());
    settings.html_template = Some("{{#if".into());
    let result = generate_from_sources_sync(settings).unwrap();
    result.collect_outputs().unwrap();
    assert!(result.shared.get().is_none());
}

#[test]
fn updates_match_fresh_output_and_failures_preserve_the_receiver() {
    let mut result = generate_from_sources_sync(options()).unwrap();
    let old_css = result.generate_css().unwrap();
    result
        .regenerate_sync(membership(&["opaque://not/a/file.svg"]), vec![])
        .unwrap();
    assert!(Arc::ptr_eq(&old_css, &result.generate_css().unwrap()));
    let old_font = result.ttf_bytes().unwrap().to_vec();
    assert!(
        result
            .regenerate_sync(
                membership(&["opaque://not/a/file.svg"]),
                vec![SourceChange::Changed {
                    id: "opaque://not/a/file.svg".into(),
                    svg: "invalid".into(),
                    name: None,
                }]
            )
            .is_err()
    );
    assert_eq!(result.ttf_bytes().unwrap(), old_font);
    assert!(Arc::ptr_eq(&old_css, &result.generate_css().unwrap()));
    result
        .regenerate_sync(
            membership(&["new", "opaque://not/a/file.svg"]),
            vec![
                SourceChange::Added(source("new", "triangle", OTHER)),
                SourceChange::Changed {
                    id: "opaque://not/a/file.svg".into(),
                    svg: OTHER.into(),
                    name: Some("renamed".into()),
                },
            ],
        )
        .unwrap();
    let mut fresh_options = options();
    fresh_options.sources = vec![
        source("new", "triangle", OTHER),
        source("opaque://not/a/file.svg", "renamed", OTHER),
    ];
    let fresh = generate_from_sources_sync(fresh_options).unwrap();
    assert_fonts_equal(&result, &fresh);
    assert_eq!(
        result.generate_css().unwrap(),
        fresh.generate_css().unwrap()
    );
    result
        .regenerate_sync(
            membership(&["new"]),
            vec![SourceChange::Removed {
                id: "opaque://not/a/file.svg".into(),
            }],
        )
        .unwrap();
    let mut fresh_options = options();
    fresh_options.sources = vec![source("new", "triangle", OTHER)];
    assert_fonts_equal(&result, &generate_from_sources_sync(fresh_options).unwrap());
}

fn variant_options() -> GenerateWebfontsFromSourcesOptions {
    GenerateWebfontsFromSourcesOptions {
        variants: Some(vec![
            SourceFontVariant {
                name: "light".into(),
                sources: vec![source("shared", "square", SVG)],
                weight: Some(300),
                default: Some(true),
            },
            SourceFontVariant {
                name: "bold".into(),
                sources: vec![source("shared", "square", SVG)],
                weight: Some(700),
                default: None,
            },
        ]),
        types: Some(vec![FontType::Ttf, FontType::Woff, FontType::Woff2]),
        incremental: Some(true),
        format_options: options().format_options,
        ..Default::default()
    }
}

#[test]
fn shared_variant_identity_is_validated_and_updates_all_memberships() {
    let mut settings = variant_options();
    let mut result = generate_from_sources_sync(settings.clone()).unwrap();
    assert!(Arc::ptr_eq(
        &result.options.sources[0][0].contents,
        &result.options.sources[1][0].contents
    ));
    result
        .regenerate_sync(
            SourceMembership::Variants(
                ["light", "bold"]
                    .map(|name| VariantSourceSet {
                        name: name.into(),
                        ids: vec!["shared".into()],
                    })
                    .into(),
            ),
            vec![SourceChange::Changed {
                id: "shared".into(),
                svg: OTHER.into(),
                name: None,
            }],
        )
        .unwrap();
    for variant in settings.variants.as_mut().unwrap() {
        variant.sources[0].svg = OTHER.into();
    }
    let fresh = generate_from_sources_sync(settings.clone()).unwrap();
    assert_fonts_equal(&result, &fresh);
    assert_eq!(
        result.generate_css().unwrap(),
        fresh.generate_css().unwrap()
    );
    settings.variants.as_mut().unwrap()[1].sources[0].name = "conflicting".into();
    assert!(
        generate_from_sources_sync(settings)
            .err()
            .unwrap()
            .to_string()
            .contains("Conflicting")
    );
}

#[test]
fn single_variant_actions_validate_targets_and_update_every_consumer() {
    let mut settings = variant_options();
    let mut result = generate_from_sources_sync(settings.clone()).unwrap();
    let expected = generate_from_sources_sync(settings.clone()).unwrap();
    let error = result
        .add_sync(source("", "new", SVG), Some("light"))
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "Source ID/path must not be empty");
    assert_fonts_equal(&result, &expected);
    let error = result
        .add_sync(source("extra", "extra", SVG), None)
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Adding to a family requires a target variant"
    );
    assert_fonts_equal(&result, &expected);
    let error = result
        .add_sync(source("extra", "extra", SVG), Some("missing"))
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "Unknown target variant");
    assert_fonts_equal(&result, &expected);
    result
        .add_sync(source("extra", "extra", SVG), Some("light"))
        .unwrap();
    settings.variants.as_mut().unwrap()[0]
        .sources
        .push(source("extra", "extra", SVG));
    assert_fonts_equal(
        &result,
        &generate_from_sources_sync(settings.clone()).unwrap(),
    );
    result
        .update_sync("shared".into(), OTHER.into(), None)
        .unwrap();
    for variant in settings.variants.as_mut().unwrap() {
        variant.sources[0].svg = OTHER.into();
    }
    let expected = generate_from_sources_sync(settings.clone()).unwrap();
    assert_fonts_equal(&result, &expected);
    let error = result.remove_sync("shared".into()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Each design must contain at least one source"
    );
    assert_fonts_equal(&result, &expected);
    result.remove_sync("extra".into()).unwrap();
    settings.variants.as_mut().unwrap()[0].sources.pop();
    assert_fonts_equal(&result, &generate_from_sources_sync(settings).unwrap());
    let mut ordinary = generate_from_sources_sync(options()).unwrap();
    let expected = generate_from_sources_sync(options()).unwrap();
    let error = ordinary
        .add_sync(source("extra", "extra", SVG), Some("light"))
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Ordinary fonts do not accept a target variant"
    );
    assert_fonts_equal(&ordinary, &expected);
}

#[tokio::test]
#[cfg(feature = "async")]
async fn async_generation_and_failed_updates_are_recoverable() {
    let result = generate_from_sources(options()).await.unwrap();
    let sync = generate_from_sources_sync(options()).unwrap();
    assert_fonts_equal(&result, &sync);
    let error = result
        .regenerate(membership(&["unknown"]), vec![])
        .await
        .err()
        .unwrap();
    let mut result = error.into_result().unwrap();
    result
        .regenerate_sync(membership(&["opaque://not/a/file.svg"]), vec![])
        .unwrap();
    assert_fonts_equal(&result, &sync);
}

#[test]
fn invalid_source_batches_are_rejected_and_fingerprints_include_identity_and_templates() {
    let mut settings = options();
    settings.sources.push(settings.sources[0].clone());
    assert!(generate_from_sources_sync(settings).is_err());
    let mut settings = options();
    settings.sources[0].id.clear();
    assert!(generate_from_sources_sync(settings).is_err());
    let baseline = generate_from_sources_sync(options())
        .unwrap()
        .generate_css()
        .unwrap();
    let mut settings = options();
    settings.sources[0].id = "other-id".into();
    assert_ne!(
        baseline,
        generate_from_sources_sync(settings)
            .unwrap()
            .generate_css()
            .unwrap()
    );
    let mut settings = options();
    settings.html_template = Some("preview".into());
    assert_ne!(
        baseline,
        generate_from_sources_sync(settings)
            .unwrap()
            .generate_css()
            .unwrap()
    );
}

#[test]
#[cfg(feature = "async")]
fn async_calls_without_a_runtime_return_errors_and_preserve_update_state() {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(Err(error)) = pin!(generate_from_sources(options())).poll(&mut context) else {
        panic!("generation without a runtime must fail immediately");
    };
    assert!(error.to_string().contains("Tokio runtime"));

    let result = generate_from_sources_sync(options()).unwrap();
    let font = result.fonts.ttf_font.clone().unwrap();
    let text = result.options.sources[0][0].contents.clone();
    let parsed = result.caches.as_ref().unwrap().glyphs.parse_count;
    let css = result.generate_css().unwrap();
    let update = || {
        vec![SourceChange::Changed {
            id: "opaque://not/a/file.svg".into(),
            name: Some("renamed".into()),
            svg: SVG.into(),
        }]
    };
    let Poll::Ready(Err(error)) =
        pin!(result.regenerate(membership(&["opaque://not/a/file.svg"]), update(),))
            .poll(&mut context)
    else {
        panic!("update without a runtime must return its receiver immediately");
    };
    assert!(
        error
            .to_string()
            .contains("Source update requires a Tokio runtime")
    );
    assert_eq!(error.to_string(), error.error().to_string());
    assert_eq!(
        std::error::Error::source(&error).unwrap().to_string(),
        error.error().to_string()
    );
    assert!(format!("{error:?}").contains("recoverable: true"));
    let (cause, recovered) = error.into_parts();
    assert!(cause.to_string().contains("Tokio runtime"));
    let mut recovered = recovered.unwrap();
    assert!(Arc::ptr_eq(
        &font,
        recovered.fonts.ttf_font.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &text,
        &recovered.options.sources[0][0].contents
    ));
    assert!(Arc::ptr_eq(&css, &recovered.generate_css().unwrap()));
    assert_eq!(
        recovered.caches.as_ref().unwrap().glyphs.parse_count,
        parsed
    );
    recovered
        .regenerate_sync(membership(&["opaque://not/a/file.svg"]), update())
        .unwrap();
    let mut settings = options();
    settings.sources[0].name = "renamed".into();
    assert_fonts_equal(&recovered, &generate_from_sources_sync(settings).unwrap());
}

#[test]
fn rejects_empty_or_mixed_inputs_and_updates_without_incremental_mode() {
    let mut empty = options();
    empty.sources.clear();
    let mut mixed = variant_options();
    mixed.sources = options().sources;
    for (settings, message) in [
        (empty, "options.sources is empty"),
        (
            mixed,
            "options.sources must be empty when variants are supplied",
        ),
    ] {
        let error = generate_from_sources_sync(settings).err().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), message);
    }
    let mut settings = options();
    settings.incremental = Some(false);
    let mut result = generate_from_sources_sync(settings).unwrap();
    let font = result.fonts.ttf_font.clone().unwrap();
    let error = result
        .regenerate_sync(membership(&["opaque://not/a/file.svg"]), vec![])
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Source updates require incremental generation"
    );
    assert!(Arc::ptr_eq(&font, result.fonts.ttf_font.as_ref().unwrap()));
}

#[test]
fn incomplete_or_reordered_variant_membership_preserves_state_for_retry() {
    let mut result = generate_from_sources_sync(variant_options()).unwrap();
    let font = result.fonts.ttf_font.clone().unwrap();
    let css = result.generate_css().unwrap();
    let parsed = result
        .caches
        .as_ref()
        .unwrap()
        .variants
        .parsed
        .iter()
        .map(|cache| cache.parse_count)
        .collect::<Vec<_>>();
    let sets = |names: &[&str]| {
        SourceMembership::Variants(
            names
                .iter()
                .map(|name| VariantSourceSet {
                    name: (*name).into(),
                    ids: vec!["shared".into()],
                })
                .collect(),
        )
    };
    for names in [
        vec!["light"],
        vec!["bold", "light"],
        vec!["light", "unknown"],
    ] {
        let error = result.regenerate_sync(sets(&names), vec![]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Supply every variant in its original order"
        );
        assert!(Arc::ptr_eq(&font, result.fonts.ttf_font.as_ref().unwrap()));
        assert!(Arc::ptr_eq(&css, &result.generate_css().unwrap()));
        assert_eq!(
            result
                .caches
                .as_ref()
                .unwrap()
                .variants
                .parsed
                .iter()
                .map(|cache| cache.parse_count)
                .collect::<Vec<_>>(),
            parsed
        );
    }
    result
        .regenerate_sync(
            sets(&["light", "bold"]),
            vec![SourceChange::Changed {
                id: "shared".into(),
                name: None,
                svg: OTHER.into(),
            }],
        )
        .unwrap();
    let mut settings = variant_options();
    for variant in settings.variants.as_mut().unwrap() {
        variant.sources[0].svg = OTHER.into();
    }
    assert_fonts_equal(&result, &generate_from_sources_sync(settings).unwrap());
}

#[test]
fn memory_fingerprints_distinguish_missing_glyph_policies() {
    use crate::{MissingGlyphBehavior, MissingGlyphOptions};

    let mut fingerprints = std::collections::HashSet::new();
    let baseline = generate_from_sources_sync(variant_options()).unwrap();
    for behavior in [
        MissingGlyphBehavior::Blank,
        MissingGlyphBehavior::Error,
        MissingGlyphBehavior::Fallback,
    ] {
        let mut settings = variant_options();
        settings.missing_glyphs = Some(MissingGlyphOptions {
            behavior,
            variant: (behavior == MissingGlyphBehavior::Fallback).then(|| "light".into()),
        });
        let result = generate_from_sources_sync(settings.clone()).unwrap();
        // Complete designs encode the same fonts, but policy is part of the generation identity.
        assert_fonts_equal(&baseline, &result);
        assert!(fingerprints.insert(result.shared().hash.clone()));
        assert_eq!(
            result.generate_css().unwrap(),
            generate_from_sources_sync(settings)
                .unwrap()
                .generate_css()
                .unwrap()
        );
    }
}

#[test]
fn unsafe_names_are_rejected_on_input_and_updates_and_unicode_is_supported() {
    for name in ["", "a b", "a\"b", "<script>", "a.b", "a:b", "a/b", "x\n"] {
        let mut settings = options();
        settings.sources[0].name = name.into();
        assert!(generate_from_sources_sync(settings).is_err(), "{name:?}");
        let mut result = generate_from_sources_sync(options()).unwrap();
        let parsed = result.caches.as_ref().unwrap().glyphs.parse_count;
        assert!(
            result
                .regenerate_sync(
                    membership(&["opaque://not/a/file.svg"]),
                    vec![SourceChange::Changed {
                        id: "opaque://not/a/file.svg".into(),
                        svg: SVG.into(),
                        name: Some(name.into()),
                    }]
                )
                .is_err()
        );
        result
            .regenerate_sync(membership(&["opaque://not/a/file.svg"]), vec![])
            .unwrap();
        assert_eq!(result.caches.as_ref().unwrap().glyphs.parse_count, parsed);
    }
    let mut settings = options();
    settings.sources[0].name = "図形-é_42".into();
    let result = generate_from_sources_sync(settings).unwrap();
    assert!(result.generate_css().unwrap().contains("図形-é_42"));
    assert!(result.generate_html().unwrap().contains("図形-é_42"));
}

#[test]
fn rename_reorder_and_identical_content_reuse_geometry_and_insensitive_renders() {
    let mut settings = options();
    settings.css_template = Some("{{fontName}}".into());
    settings.html_template = Some("{{{styles}}}".into());
    settings.sources.push(source("second", "second", OTHER));
    let mut result = generate_from_sources_sync(settings.clone()).unwrap();
    let css = result.generate_css().unwrap();
    let html = result.generate_html().unwrap();
    let parsed = result.caches.as_ref().unwrap().glyphs.parse_count;
    let unchanged_text = result.options.sources[0][1].contents.clone();
    result
        .regenerate_sync(
            membership(&["second", "opaque://not/a/file.svg"]),
            vec![SourceChange::Changed {
                id: "opaque://not/a/file.svg".into(),
                svg: OTHER.into(),
                name: Some("renamed".into()),
            }],
        )
        .unwrap();
    assert_eq!(result.caches.as_ref().unwrap().glyphs.parse_count, parsed);
    assert!(Arc::ptr_eq(
        &unchanged_text,
        &result.options.sources[0][0].contents
    ));
    assert!(Arc::ptr_eq(&css, &result.generate_css().unwrap()));
    assert!(Arc::ptr_eq(&html, &result.generate_html().unwrap()));
    settings.sources = vec![
        source("second", "second", OTHER),
        source("opaque://not/a/file.svg", "renamed", OTHER),
    ];
    assert_fonts_equal(&result, &generate_from_sources_sync(settings).unwrap());
    let ttf = result.fonts.ttf_font.clone().unwrap();
    result
        .regenerate_sync(
            membership(&["second", "opaque://not/a/file.svg"]),
            vec![SourceChange::Changed {
                id: "second".into(),
                svg: OTHER.into(),
                name: None,
            }],
        )
        .unwrap();
    assert!(Arc::ptr_eq(&ttf, result.fonts.ttf_font.as_ref().unwrap()));
}

#[test]
fn resolver_policy_and_internal_references_work_in_every_memory_mode() {
    let settings = options::resolve(options()).unwrap();
    let svg_options = svg_options_from_options(&settings.common);
    let parser = svg_options.parser_options();
    let path = crate::test_helpers::webfont_fixture("add.svg");
    assert!(
        (usvg::ImageHrefResolver::default_string_resolver())(&path, &usvg::Options::default())
            .is_some()
    );
    assert!((parser.image_href_resolver.resolve_string)(&path, &parser).is_none());
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><defs><path id="shape" d="M2 2H20V20H2Z"/></defs><use href="#shape"/><image href="{path}" width="24" height="24"/></svg>"##
    );
    let nested = (parser.image_href_resolver.resolve_data)(
        "image/svg+xml",
        Arc::new(svg.as_bytes().to_vec()),
        &parser,
    );
    assert!(matches!(nested, Some(usvg::ImageKind::SVG(_))));
    let data = svg
        .bytes()
        .map(|byte| format!("%{byte:02X}"))
        .collect::<String>();
    let svg = svg.replace(
        "</svg>",
        &format!(r#"<image href="data:image/svg+xml,{data}" width="24" height="24"/></svg>"#),
    );
    for incremental in [false, true] {
        let mut settings = options();
        settings.incremental = Some(incremental);
        settings.sources[0].svg = svg.clone();
        let result = generate_from_sources_sync(settings).unwrap();
        assert!(
            result
                .svg_string()
                .unwrap()
                .contains("glyph-name=\"square\"")
        );
        assert!(result.generate_html().is_ok());
        let mut settings = options();
        settings.types = Some(vec![FontType::Ttf]);
        settings.color_glyphs = Some(crate::ColorGlyphSelection::All);
        settings.incremental = Some(incremental);
        settings.sources[0].svg = svg.clone();
        assert!(
            generate_from_sources_sync(settings)
                .unwrap()
                .ttf_bytes()
                .is_some()
        );
    }
    let mut settings = variant_options();
    settings.color_glyphs = Some(crate::ColorGlyphSelection::All);
    for variant in settings.variants.as_mut().unwrap() {
        variant.sources[0].svg = svg.clone();
    }
    let mut result = generate_from_sources_sync(settings).unwrap();
    result
        .regenerate_sync(
            SourceMembership::Variants(
                ["light", "bold"]
                    .map(|name| VariantSourceSet {
                        name: name.into(),
                        ids: vec!["shared".into()],
                    })
                    .into(),
            ),
            vec![SourceChange::Changed {
                id: "shared".into(),
                svg: svg.replace("H20", "H22"),
                name: None,
            }],
        )
        .unwrap();
}

#[tokio::test]
#[cfg(feature = "async")]
async fn worker_panics_are_contextual_and_not_reported_as_recoverable() {
    let join = tokio::task::spawn_blocking(|| panic!("generation worker"))
        .await
        .unwrap_err();
    assert!(
        generation_join_error(join)
            .to_string()
            .contains("Source generation task failed")
    );
    let join = tokio::task::spawn_blocking(|| panic!("update worker"))
        .await
        .unwrap_err();
    let error = SourceRegenerateError::worker(join);
    assert!(
        error
            .error()
            .to_string()
            .contains("Source update task failed")
    );
    assert!(error.into_result().is_none());
}

#[test]
fn color_selection_is_fixed_and_identical_text_keeps_separate_parse_modes() {
    let mut settings = options();
    settings.types = Some(vec![FontType::Ttf, FontType::Woff, FontType::Woff2]);
    settings.color_glyphs = Some(crate::ColorGlyphSelection::Named(vec!["square".into()]));
    settings.sources.push(source("mono", "mono", SVG));
    let mut result = generate_from_sources_sync(settings.clone()).unwrap();
    let cache = &result.caches.as_ref().unwrap().glyphs;
    assert!(
        cache.entries["opaque://not/a/file.svg"]
            .color_layers
            .is_some()
    );
    assert!(cache.entries["mono"].color_layers.is_none());
    let parsed = cache.parse_count;
    assert!(
        result
            .regenerate_sync(
                membership(&["mono"]),
                vec![SourceChange::Removed {
                    id: "opaque://not/a/file.svg".into()
                }]
            )
            .is_err()
    );
    assert_eq!(result.caches.as_ref().unwrap().glyphs.parse_count, parsed);
    assert!(
        result
            .regenerate_sync(
                membership(&["opaque://not/a/file.svg", "mono"]),
                vec![SourceChange::Changed {
                    id: "opaque://not/a/file.svg".into(),
                    svg: SVG.into(),
                    name: Some("renamed".into()),
                }]
            )
            .is_err()
    );
    assert_eq!(result.caches.as_ref().unwrap().glyphs.parse_count, parsed);
    result
        .regenerate_sync(
            membership(&["opaque://not/a/file.svg", "mono"]),
            vec![SourceChange::Changed {
                id: "opaque://not/a/file.svg".into(),
                svg: OTHER.into(),
                name: None,
            }],
        )
        .unwrap();
    settings.sources[0].svg = OTHER.into();
    assert_fonts_equal(&result, &generate_from_sources_sync(settings).unwrap());
    assert_eq!(
        result.caches.as_ref().unwrap().glyphs.parse_count,
        parsed + 1
    );
}

#[test]
#[cfg(feature = "fs")]
fn variant_text_generation_matches_files_including_color() {
    let path = crate::test_helpers::webfont_fixture("add.svg");
    let svg = std::fs::read_to_string(&path).unwrap();
    for color in [None, Some(crate::ColorGlyphSelection::All)] {
        let mut settings = variant_options();
        settings.color_glyphs = color.clone();
        for variant in settings.variants.as_mut().unwrap() {
            variant.sources = vec![source("shared", "add", &svg)];
        }
        let file = crate::generate_sync(
            GenerateWebfontsOptions {
                dest: "unused-output".into(),
                variants: Some(
                    settings
                        .variants
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|variant| crate::FontVariant {
                            name: variant.name.clone(),
                            files: vec![path.clone()],
                            weight: variant.weight,
                            default: variant.default,
                        })
                        .collect(),
                ),
                color_glyphs: color,
                types: settings.types.clone(),
                format_options: settings.format_options.clone(),
                write_files: Some(false),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        let memory = generate_from_sources_sync(settings).unwrap();
        assert_eq!(memory.ttf_bytes(), file.ttf_bytes());
        assert_eq!(memory.woff_bytes(), file.woff_bytes());
        assert_eq!(memory.woff2_bytes(), file.woff2_bytes());
    }
}

#[tokio::test]
#[cfg(feature = "async")]
async fn variant_parse_failure_recovers_and_retry_matches_fresh_generation() {
    let result = generate_from_sources(variant_options()).await.unwrap();
    let sets = || {
        SourceMembership::Variants(
            ["light", "bold"]
                .map(|name| VariantSourceSet {
                    name: name.into(),
                    ids: vec!["shared".into()],
                })
                .into(),
        )
    };
    let error = result
        .regenerate(
            sets(),
            vec![SourceChange::Changed {
                id: "shared".into(),
                svg: "broken".into(),
                name: None,
            }],
        )
        .await
        .err()
        .unwrap();
    assert!(error.error().to_string().contains("shared"));
    let result = error
        .into_result()
        .unwrap()
        .regenerate(
            sets(),
            vec![SourceChange::Changed {
                id: "shared".into(),
                svg: OTHER.into(),
                name: None,
            }],
        )
        .await
        .unwrap();
    let mut settings = variant_options();
    for variant in settings.variants.as_mut().unwrap() {
        variant.sources[0].svg = OTHER.into();
    }
    assert_fonts_equal(&result, &generate_from_sources_sync(settings).unwrap());
}

#[test]
fn invalid_variant_metadata_preserves_warm_caches_before_a_real_retry() {
    let mut settings = variant_options();
    settings.missing_glyphs = Some(crate::MissingGlyphOptions {
        behavior: crate::MissingGlyphBehavior::Error,
        variant: None,
    });
    let mut result = generate_from_sources_sync(settings).unwrap();
    let parsed = result
        .caches
        .as_ref()
        .unwrap()
        .variants
        .parsed
        .iter()
        .map(|cache| cache.parse_count)
        .sum::<usize>();
    let sets = SourceMembership::Variants(vec![
        VariantSourceSet {
            name: "light".into(),
            ids: vec!["shared".into(), "new".into()],
        },
        VariantSourceSet {
            name: "bold".into(),
            ids: vec!["shared".into()],
        },
    ]);
    assert!(
        result
            .regenerate_sync(
                sets,
                vec![SourceChange::Added(source("new", "other", OTHER))]
            )
            .is_err()
    );
    assert_eq!(
        result
            .caches
            .as_ref()
            .unwrap()
            .variants
            .parsed
            .iter()
            .map(|cache| cache.parse_count)
            .sum::<usize>(),
        parsed
    );
    let sets = SourceMembership::Variants(
        ["light", "bold"]
            .map(|name| VariantSourceSet {
                name: name.into(),
                ids: vec!["shared".into()],
            })
            .into(),
    );
    result
        .regenerate_sync(
            sets,
            vec![SourceChange::Changed {
                id: "shared".into(),
                name: Some("renamed".into()),
                svg: SVG.into(),
            }],
        )
        .unwrap();
    assert_eq!(
        result
            .caches
            .as_ref()
            .unwrap()
            .variants
            .parsed
            .iter()
            .map(|cache| cache.parse_count)
            .sum::<usize>(),
        parsed
    );
}

#[test]
fn variant_add_rename_reorder_and_remove_match_fresh_builds() {
    let mut settings = variant_options();
    let mut result = generate_from_sources_sync(settings.clone()).unwrap();
    let sets = |settings: &GenerateWebfontsFromSourcesOptions| {
        SourceMembership::Variants(
            settings
                .variants
                .as_ref()
                .unwrap()
                .iter()
                .map(|variant| VariantSourceSet {
                    name: variant.name.clone(),
                    ids: variant
                        .sources
                        .iter()
                        .map(|source| source.id.clone())
                        .collect(),
                })
                .collect(),
        )
    };
    let assert_fresh = |result: &GenerateWebfontsFromSourcesResult,
                        settings: &GenerateWebfontsFromSourcesOptions| {
        let fresh = generate_from_sources_sync(settings.clone()).unwrap();
        assert_fonts_equal(result, &fresh);
        assert_eq!(
            result.generate_css().unwrap(),
            fresh.generate_css().unwrap()
        );
        assert_eq!(
            result.generate_html().unwrap(),
            fresh.generate_html().unwrap()
        );
    };
    for variant in settings.variants.as_mut().unwrap() {
        variant.sources.push(source("new", "triangle", OTHER));
    }
    result
        .regenerate_sync(
            sets(&settings),
            vec![SourceChange::Added(source("new", "triangle", OTHER))],
        )
        .unwrap();
    assert_fresh(&result, &settings);
    for variant in settings.variants.as_mut().unwrap() {
        variant.sources.reverse();
        variant.sources[1].name = "renamed".into();
    }
    result
        .regenerate_sync(
            sets(&settings),
            vec![SourceChange::Changed {
                id: "shared".into(),
                svg: SVG.into(),
                name: Some("renamed".into()),
            }],
        )
        .unwrap();
    assert_fresh(&result, &settings);
    // A design-only removal exercises the shared missing-glyph matrix and retained ID definition.
    settings.variants.as_mut().unwrap()[1].sources.pop();
    result.regenerate_sync(sets(&settings), vec![]).unwrap();
    assert_fresh(&result, &settings);
    settings.variants.as_mut().unwrap()[0].sources.pop();
    result
        .regenerate_sync(
            sets(&settings),
            vec![SourceChange::Removed {
                id: "shared".into(),
            }],
        )
        .unwrap();
    assert_fresh(&result, &settings);
}

#[test]
fn rejected_batches_preserve_sources_payloads_and_reusable_caches() {
    let mut result = generate_from_sources_sync(options()).unwrap();
    let id = "opaque://not/a/file.svg";
    let before = result.fonts.ttf_font.clone().unwrap();
    let text = result.options.sources[0][0].contents.clone();
    let parsed = result.caches.as_ref().unwrap().glyphs.parse_count;
    let bad_batches = [
        (membership(&[]), vec![]),
        (
            membership(&[id]),
            vec![SourceChange::Added(source(id, "square", SVG))],
        ),
        (
            membership(&[id]),
            vec![SourceChange::Changed {
                id: "unknown".into(),
                svg: SVG.into(),
                name: None,
            }],
        ),
        (
            membership(&[id]),
            vec![SourceChange::Removed {
                id: "unknown".into(),
            }],
        ),
        (
            membership(&[id]),
            vec![SourceChange::Removed { id: id.into() }],
        ),
        (
            membership(&[id]),
            vec![SourceChange::Added(source("orphan", "orphan", SVG))],
        ),
        (
            membership(&[id]),
            vec![
                SourceChange::Changed {
                    id: id.into(),
                    svg: OTHER.into(),
                    name: None,
                },
                SourceChange::Removed { id: id.into() },
            ],
        ),
        (
            membership(&[id, "duplicate-name"]),
            vec![SourceChange::Added(source(
                "duplicate-name",
                "square",
                OTHER,
            ))],
        ),
        (membership(&[id, id]), vec![]),
        (SourceMembership::Variants(vec![]), vec![]),
    ];
    for (members, changes) in bad_batches {
        assert!(result.regenerate_sync(members, changes).is_err());
        assert!(Arc::ptr_eq(
            &before,
            result.fonts.ttf_font.as_ref().unwrap()
        ));
        assert!(Arc::ptr_eq(&text, &result.options.sources[0][0].contents));
        assert_eq!(result.caches.as_ref().unwrap().glyphs.parse_count, parsed);
    }
    result
        .regenerate_sync(
            membership(&[id]),
            vec![SourceChange::Changed {
                id: id.into(),
                svg: SVG.into(),
                name: Some("valid-rename".into()),
            }],
        )
        .unwrap();
    assert_eq!(result.caches.as_ref().unwrap().glyphs.parse_count, parsed);
    let mut settings = options();
    settings.sources[0].name = "valid-rename".into();
    assert_fonts_equal(&result, &generate_from_sources_sync(settings).unwrap());
}

#[test]
#[cfg(feature = "fs")]
fn custom_templates_and_nondefault_settings_match_the_file_adapter() {
    let css_text = "{{fontName}}\n{{{src}}}\n{{#each codepoints}}{{@key}}={{this}};{{/each}}";
    let html_text = "{{fontName}}\n{{{styles}}}\n{{#each names}}{{this}};{{/each}}";
    let css_path = crate::test_helpers::write_temp_template("memory-parity-css", css_text);
    let html_path = crate::test_helpers::write_temp_template("memory-parity-html", html_text);
    let source_path = crate::test_helpers::webfont_fixture("add.svg");
    let file = crate::generate_sync(
        GenerateWebfontsOptions {
            files: vec![source_path.clone()],
            dest: "unused-output".into(),
            write_files: Some(false),
            css_template: Some(css_path.clone()),
            html_template: Some(html_path.clone()),
            html: Some(true),
            font_name: Some("custom-font".into()),
            font_height: Some(2048.0),
            normalize: Some(false),
            center_horizontally: Some(true),
            center_vertically: Some(true),
            ligature: Some(false),
            round: Some(2.0),
            codepoints: Some(HashMap::from([("add".into(), 0xE040)])),
            types: options().types,
            format_options: options().format_options,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let memory = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        sources: vec![source(
            "opaque-id",
            "add",
            include_str!("../svg/fixtures/unit/add.svg"),
        )],
        css_template: Some(css_text.into()),
        html_template: Some(html_text.into()),
        html: Some(true),
        font_name: Some("custom-font".into()),
        font_height: Some(2048.0),
        normalize: Some(false),
        center_horizontally: Some(true),
        center_vertically: Some(true),
        ligature: Some(false),
        round: Some(2.0),
        codepoints: Some(std::collections::HashMap::from([("add".into(), 0xE040)])),
        types: options().types,
        format_options: options().format_options,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(file.svg_string(), memory.svg_string());
    assert_eq!(file.ttf_bytes(), memory.ttf_bytes());
    assert_eq!(file.woff_bytes(), memory.woff_bytes());
    assert_eq!(file.woff2_bytes(), memory.woff2_bytes());
    assert_eq!(file.eot_bytes(), memory.eot_bytes());
    let file_css = file.generate_css_pure(None).unwrap();
    let memory_css = memory.generate_css().unwrap();
    let file_token = format!("?{}", file.get_cached_io().unwrap().shared.hash);
    let memory_token = format!("?{}", memory.shared().hash);
    assert_eq!(
        file_css.replace(&file_token, "?HASH"),
        memory_css.replace(&memory_token, "?HASH")
    );
    assert_eq!(
        file.generate_html_pure(None)
            .unwrap()
            .replace(&file_token, "?HASH"),
        memory
            .generate_html()
            .unwrap()
            .replace(&memory_token, "?HASH")
    );
    std::fs::remove_file(css_path).unwrap();
    std::fs::remove_file(html_path).unwrap();
}

fn cached(bytes: Vec<u8>, incremental: bool) -> GenerateWebfontsFromSourcesOptions {
    GenerateWebfontsFromSourcesOptions {
        sources: vec![source("a", "square", SVG), source("b", "triangle", OTHER)],
        incremental: Some(incremental),
        cache: Some(bytes),
        ..options()
    }
}

/// (parsed, processed) glyph work for an incremental result.
fn glyph_work(result: &GenerateWebfontsFromSourcesResult) -> (usize, usize) {
    let glyphs = &result.caches.as_ref().unwrap().glyphs;
    (glyphs.parse_count, glyphs.process_count)
}

/// Whether an incremental result took every font from the cache (no TTF glyph or table compiled).
fn fonts_reused(result: &GenerateWebfontsFromSourcesResult) -> bool {
    let ttf = &result.caches.as_ref().unwrap().ttf;
    ttf.compile_count == 0 && ttf.table_compile_count == 0
}

#[test]
fn cache_bytes_round_trip_into_warm_byte_identical_runs() {
    let uncached = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        cache: None,
        ..cached(Vec::new(), true)
    })
    .unwrap();
    let mut cold = generate_from_sources_sync(cached(Vec::new(), true)).unwrap();
    assert_fonts_equal(&cold, &uncached);
    assert_eq!(glyph_work(&cold), (2, 2));
    let bytes = cold.cache_bytes().unwrap();
    // Nothing changed since the last export.
    assert!(cold.cache_bytes().is_none());

    let mut warm = generate_from_sources_sync(cached(bytes.clone(), true)).unwrap();
    assert_fonts_equal(&warm, &uncached);
    assert_eq!(glyph_work(&warm), (0, 0));
    assert!(fonts_reused(&warm));
    assert!(!fonts_reused(&cold));
    assert!(warm.cache_bytes().is_none());

    // Without incremental caches, the result keeps the glyph data needed for export.
    let mut one_shot = generate_from_sources_sync(cached(Vec::new(), false)).unwrap();
    assert_eq!(one_shot.cache_bytes().unwrap(), bytes);
    let mut one_shot = generate_from_sources_sync(cached(bytes, false)).unwrap();
    assert_fonts_equal(&one_shot, &uncached);
    assert!(one_shot.cache_bytes().is_none());
}

#[test]
fn cache_bytes_follow_incremental_updates() {
    let mut result = generate_from_sources_sync(cached(Vec::new(), true)).unwrap();
    let initial = result.cache_bytes().unwrap();
    result
        .regenerate_sync(
            membership(&["a", "b"]),
            vec![SourceChange::Changed {
                id: "b".into(),
                svg: SVG.replace("H20", "H16"),
                name: None,
            }],
        )
        .unwrap();
    let updated = result.cache_bytes().unwrap();
    assert_ne!(updated, initial);

    let mut edited = cached(updated, true);
    edited.sources[1].svg = SVG.replace("H20", "H16");
    let warm = generate_from_sources_sync(edited.clone()).unwrap();
    assert_eq!(glyph_work(&warm), (0, 0));
    // Keys follow the updated glyphs, so the exported fonts are reused too.
    assert!(fonts_reused(&warm));
    edited.cache = None;
    assert_fonts_equal(&warm, &generate_from_sources_sync(edited).unwrap());
}

#[test]
fn a_warm_result_follows_edits_renames_and_additions() {
    let bytes = generate_from_sources_sync(cached(Vec::new(), true))
        .unwrap()
        .cache_bytes()
        .unwrap();
    let mut warm = generate_from_sources_sync(cached(bytes, true)).unwrap();
    assert_eq!(glyph_work(&warm), (0, 0));
    assert!(fonts_reused(&warm));
    warm.regenerate_sync(
        membership(&["a", "b", "c"]),
        vec![
            SourceChange::Changed {
                id: "a".into(),
                svg: SVG.into(),
                name: Some("box".into()),
            },
            SourceChange::Changed {
                id: "b".into(),
                svg: SVG.replace("H20", "H16"),
                name: None,
            },
            SourceChange::Added(source("c", "circle", &SVG.replace("H20", "H12"))),
        ],
    )
    .unwrap();

    let mut current = GenerateWebfontsFromSourcesOptions {
        sources: vec![
            source("a", "box", SVG),
            source("b", "triangle", &SVG.replace("H20", "H16")),
            source("c", "circle", &SVG.replace("H20", "H12")),
        ],
        ..cached(Vec::new(), true)
    };
    current.cache = None;
    assert_fonts_equal(&warm, &generate_from_sources_sync(current.clone()).unwrap());

    // The exported bytes warm a fresh run of the edited set completely.
    current.cache = warm.cache_bytes();
    assert!(current.cache.is_some());
    let rewarmed = generate_from_sources_sync(current.clone()).unwrap();
    assert_eq!(glyph_work(&rewarmed), (0, 0));
    assert!(fonts_reused(&rewarmed));
    current.cache = None;
    assert_fonts_equal(&rewarmed, &generate_from_sources_sync(current).unwrap());
}

#[test]
fn a_failed_update_never_exports_a_weaker_cache() {
    let mut result = generate_from_sources_sync(cached(Vec::new(), true)).unwrap();
    let initial = result.cache_bytes().unwrap();
    let broken = |svg: &str| {
        vec![SourceChange::Changed {
            id: "b".into(),
            svg: svg.into(),
            name: None,
        }]
    };
    assert!(
        result
            .regenerate_sync(membership(&["a", "b"]), broken("<svg"))
            .is_err()
    );
    // The failure reset the glyph data; exporting now would drop it from the stored cache.
    assert_eq!(result.cache_bytes(), None);

    // The next successful update refills it, and the export is complete again.
    let edited = SVG.replace("H20", "H16");
    result
        .regenerate_sync(membership(&["a", "b"]), broken(&edited))
        .unwrap();
    let mut next = cached(result.cache_bytes().unwrap(), true);
    assert_ne!(next.cache.as_deref(), Some(initial.as_slice()));
    next.sources[1].svg = edited;
    assert_eq!(
        glyph_work(&generate_from_sources_sync(next).unwrap()),
        (0, 0)
    );
}

#[test]
fn a_fallback_timestamp_is_used_but_kept_out_of_cache_keys() {
    let session = |fallback_ts, cache| GenerateWebfontsFromSourcesOptions {
        format_options: None,
        fallback_ts: Some(fallback_ts),
        cache,
        ..cached(Vec::new(), true)
    };
    // Without a cache it behaves exactly like `ts`.
    let mut pinned = session(1_700_000_000, None);
    pinned.fallback_ts = None;
    pinned.format_options = options().format_options;
    assert_fonts_equal(
        &generate_from_sources_sync(session(1_700_000_000, None)).unwrap(),
        &generate_from_sources_sync(pinned).unwrap(),
    );

    // A later session with another clock reuses every font, keeping the first session's
    // timestamp, and leaves the stored bytes unchanged.
    let mut first = generate_from_sources_sync(session(1_700_000_000, Some(Vec::new()))).unwrap();
    let bytes = first.cache_bytes().unwrap();
    let mut later = generate_from_sources_sync(session(1_800_000_000, Some(bytes))).unwrap();
    assert!(fonts_reused(&later));
    assert_fonts_equal(&later, &first);
    assert_eq!(later.cache_bytes(), None);
}

#[test]
fn a_corrupt_glyph_section_in_cache_bytes_is_found_when_needed() {
    let expected = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        cache: None,
        ..cached(Vec::new(), true)
    })
    .unwrap();
    let valid = generate_from_sources_sync(cached(Vec::new(), true))
        .unwrap()
        .cache_bytes()
        .unwrap();
    // Magic, format version, engine version, fingerprint, then the glyph section's frame.
    let header = 8 + 4 + 4 + env!("CARGO_PKG_VERSION").len() + 16;
    let mut corrupt = valid.clone();
    corrupt[header + 8 + 20] ^= 0xff;

    // Incremental results need the saved glyphs: a full miss that exports fresh, valid bytes.
    let mut incremental = generate_from_sources_sync(cached(corrupt.clone(), true)).unwrap();
    assert_fonts_equal(&incremental, &expected);
    assert_eq!(glyph_work(&incremental), (2, 2));
    assert_eq!(incremental.cache_bytes().unwrap(), valid);

    // A one-shot run whose fonts all hit never reads them, so nothing changes or is exported.
    let mut one_shot = generate_from_sources_sync(cached(corrupt, false)).unwrap();
    assert_fonts_equal(&one_shot, &expected);
    assert_eq!(one_shot.cache_bytes(), None);
}

#[test]
fn unusable_or_unrequested_cache_bytes_are_ignored() {
    let expected = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        cache: None,
        ..cached(Vec::new(), true)
    })
    .unwrap();
    let valid = generate_from_sources_sync(cached(Vec::new(), true))
        .unwrap()
        .cache_bytes()
        .unwrap();

    let mut wrong_version = valid.clone();
    wrong_version[8] ^= 0xff;
    // Valid header, corrupted body: a flipped byte inside the last stored font still parses, so
    // only the checksum can catch it.
    let mut flipped_payload = valid.clone();
    let last = flipped_payload.len() - 10;
    flipped_payload[last] ^= 0xff;
    let mut trailing = valid.clone();
    trailing.extend_from_slice(b"extra");
    for bytes in [
        b"not a cache".to_vec(),
        Vec::new(),
        wrong_version,
        valid[..valid.len() / 2].to_vec(),
        flipped_payload,
        trailing,
    ] {
        let mut result = generate_from_sources_sync(cached(bytes, true)).unwrap();
        assert_fonts_equal(&result, &expected);
        assert_eq!(glyph_work(&result), (2, 2));
        // The run exports a fresh, valid cache in place of the unusable bytes.
        assert_eq!(result.cache_bytes().unwrap(), valid);
    }

    let mut uncached = generate_from_sources_sync(options()).unwrap();
    assert!(uncached.cache_bytes().is_none());
}

const COLOR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#ff9900" d="M2 2H18V20H2Z"/><path fill="#0066ff" d="M6 6H12V12H6Z"/></svg>"##;

/// A two-design family with a design-specific glyph, a shared one, and a color glyph.
fn cached_family(bytes: Option<Vec<u8>>, incremental: bool) -> GenerateWebfontsFromSourcesOptions {
    let mut options = variant_options();
    let designs = options.variants.as_mut().unwrap();
    designs[0]
        .sources
        .push(source("light-b", "triangle", OTHER));
    designs[1].sources.push(source(
        "bold-b",
        "triangle",
        &OTHER.replace("24 24H0", "20 24H0"),
    ));
    for design in designs.iter_mut() {
        design.sources.push(source("logo", "logo", COLOR));
    }
    GenerateWebfontsFromSourcesOptions {
        color_glyphs: Some(crate::ColorGlyphSelection::Named(vec!["logo".into()])),
        incremental: Some(incremental),
        cache: bytes,
        ..options
    }
}

#[test]
fn palette_slot_changes_keep_saved_glyphs_in_memory() {
    let slots = |color: &str| {
        Some(std::collections::HashMap::from([(
            "brand".to_owned(),
            color.to_owned(),
        )]))
    };
    let check = |mut options: GenerateWebfontsFromSourcesOptions,
                 work: fn(&GenerateWebfontsFromSourcesResult) -> (usize, usize)| {
        options.color_slots = slots("#ff9900");
        options.cache = Some(Vec::new());
        options.cache = generate_from_sources_sync(options.clone())
            .unwrap()
            .cache_bytes();
        assert!(options.cache.is_some());
        // Slots only feed the color tables: every saved glyph is reused, the fonts are rebuilt.
        options.color_slots = slots("#0066ff");
        let changed = generate_from_sources_sync(options.clone()).unwrap();
        assert_eq!(work(&changed), (0, 0));
        options.cache = None;
        assert_fonts_equal(&changed, &generate_from_sources_sync(options).unwrap());
    };
    let mut single = cached(Vec::new(), true);
    single.sources.push(source("logo", "logo", COLOR));
    single.color_glyphs = Some(crate::ColorGlyphSelection::Named(vec!["logo".into()]));
    single.types = Some(vec![FontType::Ttf, FontType::Woff2]);
    check(single, glyph_work);
    check(cached_family(None, true), variant_work);
}

/// (parsed, processed) variant glyph work for an incremental result.
fn variant_work(result: &GenerateWebfontsFromSourcesResult) -> (usize, usize) {
    let variants = &result.caches.as_ref().unwrap().variants;
    let parsed = variants
        .parsed
        .iter()
        .map(|design| design.parse_count)
        .sum();
    (parsed, variants.process_count)
}

#[test]
fn variant_cache_bytes_round_trip_and_follow_updates() {
    let uncached = generate_from_sources_sync(cached_family(None, true)).unwrap();
    let mut cold = generate_from_sources_sync(cached_family(Some(Vec::new()), true)).unwrap();
    assert_fonts_equal(&cold, &uncached);
    assert_eq!(variant_work(&cold), (6, 6));
    let bytes = cold.cache_bytes().unwrap();
    assert!(cold.cache_bytes().is_none());
    // Without incremental caches, the result keeps the variant glyph data needed for export.
    let mut one_shot = generate_from_sources_sync(cached_family(Some(Vec::new()), false)).unwrap();
    assert_fonts_equal(&one_shot, &uncached);
    assert_eq!(one_shot.cache_bytes().unwrap(), bytes);

    let mut warm = generate_from_sources_sync(cached_family(Some(bytes.clone()), true)).unwrap();
    assert_fonts_equal(&warm, &uncached);
    assert_eq!(variant_work(&warm), (0, 0));
    assert!(warm.cache_bytes().is_none());
    // A settings change that only feeds CSS reuses every font: nothing was prepared or retained.
    let mut css = cached_family(Some(bytes.clone()), false);
    css.css_fonts_url = Some("/fonts/".into());
    let mut css = generate_from_sources_sync(css).unwrap();
    assert_fonts_equal(&css, &uncached);
    assert!(css.persistent.is_none());
    assert!(css.cache_bytes().is_none());

    let changed = SVG.replace("H20", "H16");
    warm.update_sync("shared".into(), changed.clone(), None)
        .unwrap();
    let mut edited = cached_family(None, true);
    for design in edited.variants.as_mut().unwrap() {
        design.sources[0].svg.clone_from(&changed);
    }
    assert_fonts_equal(&warm, &generate_from_sources_sync(edited.clone()).unwrap());
    let updated = warm.cache_bytes().unwrap();
    assert_ne!(updated, bytes);

    edited.cache = Some(updated);
    let mut rewarmed = generate_from_sources_sync(edited.clone()).unwrap();
    assert_eq!(variant_work(&rewarmed), (0, 0));
    edited.cache = None;
    assert_fonts_equal(&rewarmed, &generate_from_sources_sync(edited).unwrap());
    assert!(rewarmed.cache_bytes().is_none());
}

#[test]
fn variant_families_with_no_requested_formats_still_validate_sources() {
    let broken = |cache| {
        let mut options = cached_family(cache, false);
        options.types = Some(Vec::new());
        let bold = &mut options.variants.as_mut().unwrap()[1].sources;
        bold.iter_mut()
            .find(|source| source.id == "bold-b")
            .unwrap()
            .svg = "<svg".into();
        generate_from_sources_sync(options)
            .err()
            .unwrap()
            .to_string()
    };
    // An empty format list is never a cache hit, so the family is prepared as without a cache.
    assert_eq!(broken(Some(Vec::new())), broken(None));
}

#[test]
fn unusable_variant_cache_bytes_fall_back_and_are_replaced() {
    let expected = generate_from_sources_sync(cached_family(None, true)).unwrap();
    let valid = generate_from_sources_sync(cached_family(Some(Vec::new()), true))
        .unwrap()
        .cache_bytes()
        .unwrap();
    // A single-design cache for the same font name has another identity.
    let single = generate_from_sources_sync(cached(Vec::new(), true))
        .unwrap()
        .cache_bytes()
        .unwrap();

    let mut wrong_version = valid.clone();
    wrong_version[8] ^= 0xff;
    let mut flipped_payload = valid.clone();
    let last = flipped_payload.len() - 10;
    flipped_payload[last] ^= 0xff;
    let mut trailing = valid.clone();
    trailing.extend_from_slice(b"extra");
    for bytes in [
        b"not a cache".to_vec(),
        Vec::new(),
        wrong_version,
        valid[..valid.len() / 2].to_vec(),
        flipped_payload,
        trailing,
        single,
    ] {
        let mut result = generate_from_sources_sync(cached_family(Some(bytes), true)).unwrap();
        assert_fonts_equal(&result, &expected);
        assert_eq!(variant_work(&result), (6, 6));
        assert_eq!(result.cache_bytes().unwrap(), valid);
    }
}

#[cfg(feature = "fs")]
#[test]
fn memory_cache_bytes_are_the_file_cache_format() {
    let dir = std::env::temp_dir()
        .join(format!("wfg-memory-file-cache-{}", std::process::id()))
        .to_string_lossy()
        .into_owned();
    std::fs::create_dir_all(&dir).unwrap();
    let svg_path = std::path::Path::new(&dir).join("square.svg");
    std::fs::write(&svg_path, SVG).unwrap();
    let cache_dir = std::path::Path::new(&dir).join("cache");
    let file_options = GenerateWebfontsOptions {
        dest: dir.clone(),
        files: vec![svg_path.to_string_lossy().into_owned()],
        types: options().types,
        format_options: options().format_options,
        write_files: Some(false),
        cache_location: Some(cache_dir.to_string_lossy().into_owned()),
        ..Default::default()
    };
    crate::generate_sync(file_options, None).unwrap();
    let file = std::fs::read_dir(&cache_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();

    let warm = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        sources: vec![source("x", "square", SVG)],
        cache: Some(std::fs::read(file).unwrap()),
        ..options()
    })
    .unwrap();
    assert_eq!(glyph_work(&warm), (0, 0));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn variant_caches_follow_missing_glyph_fallback() {
    // A light-only glyph that the bold design falls back to the light artwork for.
    let mut family = cached_family(Some(Vec::new()), true);
    family.variants.as_mut().unwrap()[0].sources.push(source(
        "light-only",
        "circle",
        &SVG.replace("H20", "H12"),
    ));
    family.missing_glyphs = Some(crate::MissingGlyphOptions {
        behavior: crate::MissingGlyphBehavior::Fallback,
        variant: Some("light".into()),
    });
    let uncached = |options: &GenerateWebfontsFromSourcesOptions| {
        generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
            cache: None,
            ..options.clone()
        })
        .unwrap()
    };
    let mut cold = generate_from_sources_sync(family.clone()).unwrap();
    assert_fonts_equal(&cold, &uncached(&family));
    family.cache = cold.cache_bytes();
    let warm = generate_from_sources_sync(family.clone()).unwrap();
    assert_fonts_equal(&warm, &uncached(&family));
    assert_eq!(variant_work(&warm), (0, 0));

    // The same bytes serve a blank fallback: the glyph set records how each cell resolved.
    let mut blank = family.clone();
    blank.missing_glyphs = None;
    assert_fonts_equal(
        &generate_from_sources_sync(blank.clone()).unwrap(),
        &uncached(&blank),
    );
}

#[test]
fn variant_caches_cover_blank_glyphs_failed_updates_and_invalid_families() {
    // A light-only glyph is blank in the bold design.
    let mut sparse = cached_family(Some(Vec::new()), true);
    let light_only = SVG.replace("H20", "H12");
    sparse.variants.as_mut().unwrap()[0]
        .sources
        .push(source("light-only", "circle", &light_only));
    let uncached = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        cache: None,
        ..sparse.clone()
    })
    .unwrap();
    let mut cold = generate_from_sources_sync(sparse.clone()).unwrap();
    assert_fonts_equal(&cold, &uncached);
    sparse.cache = cold.cache_bytes();
    let mut warm = generate_from_sources_sync(sparse.clone()).unwrap();
    assert_fonts_equal(&warm, &uncached);
    assert_eq!(variant_work(&warm), (0, 0));

    // A failed update discards the glyph caches, so nothing is exported until an update succeeds.
    assert!(
        warm.update_sync("light-only".into(), "<broken".into(), None)
            .is_err()
    );
    assert_eq!(warm.cache_bytes(), None);
    let edited = SVG.replace("H20", "H16");
    warm.update_sync("light-only".into(), edited.clone(), None)
        .unwrap();
    let mut updated = sparse.clone();
    for source in updated
        .variants
        .as_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|design| &mut design.sources)
    {
        if source.id == "light-only" {
            source.svg.clone_from(&edited);
        }
    }
    updated.cache = warm.cache_bytes();
    assert!(updated.cache.is_some());
    let rewarmed = generate_from_sources_sync(updated.clone()).unwrap();
    assert_eq!(variant_work(&rewarmed), (0, 0));
    updated.cache = None;
    assert_fonts_equal(&rewarmed, &generate_from_sources_sync(updated).unwrap());

    // Family validation still fails before the cache is consulted.
    sparse.missing_glyphs = Some(crate::MissingGlyphOptions {
        behavior: crate::MissingGlyphBehavior::Error,
        variant: None,
    });
    assert!(generate_from_sources_sync(sparse).is_err());
}
