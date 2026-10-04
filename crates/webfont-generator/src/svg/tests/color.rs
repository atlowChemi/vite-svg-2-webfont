use super::*;
use crate::svg::types::{ColorSelection, ResolvedLayerPaint};
use crate::svg::{VariantGlyphCache, prepare_variant_svg_family_cached};

fn source(name: &str, root: &str, body: &str) -> LoadedSvgFile {
    LoadedSvgFile {
        path: format!("{name}.svg"),
        glyph_name: name.into(),
        contents: format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" {root}>{body}</svg>"#).into(),
    }
}

fn options(files: &[LoadedSvgFile]) -> ResolvedGenerateWebfontsOptions {
    let mut options = input::resolve_generate_webfonts_options(GenerateWebfontsOptions {
        dest: "unused".into(),
        files: files.iter().map(|f| f.path.clone()).collect(),
        types: Some(vec![FontType::Ttf]),
        write_files: Some(false),
        ..Default::default()
    })
    .unwrap();
    input::finalize_generate_webfonts_options(&mut options, files).unwrap();
    options
}

const RECT: &str = r#"<path d="M10 10H90V90H10Z"/>"#;

#[test]
fn color_pipeline_preserves_namespaced_xml_entities_and_empty_selection() {
    let mut file = source("icon", "", "");
    file.contents = r##"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY red "#ff0000">]><!-- <svg fill='wrong'> --><s:svg xmlns:s="http://www.w3.org/2000/svg" width="100" height="100" data-note="a > b"><s:path fill="&red;" d="M1 1H9V9Z"/><s:path fill="#000000" d="M10 10H20V20Z"/><s:path fill="#000001" d="M20 20H30V30Z"/><s:path d="M30 30H40V40Z"/><s:path visibility="hidden" d="M40 40H50V50Z"/></s:svg>"##.into();
    let files = vec![file];
    let resolved = options(&files);
    let mut opts = svg_options_from_options(&resolved);
    opts.color_selection = Some(&ColorSelection::All);
    let color = prepare_svg_font(&opts, &files).unwrap();
    let layers = color.processed_glyphs[0].color_layers.as_ref().unwrap();
    assert_eq!(layers.len(), 4);
    assert_eq!(
        layers[1].paint,
        ResolvedLayerPaint::Solid {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 1.0
        }
    );
    assert_eq!(
        layers[0].paint,
        ResolvedLayerPaint::Solid {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 1.0
        }
    );
    assert_eq!(
        layers[2].paint,
        ResolvedLayerPaint::Solid {
            red: 0,
            green: 0,
            blue: 1,
            alpha: 1.0
        }
    );
    assert_eq!(
        layers[3].paint,
        ResolvedLayerPaint::Foreground { alpha: 1.0 }
    );
    let empty = ColorSelection::Named(Default::default());
    opts.color_selection = Some(&empty);
    assert!(
        prepare_svg_font(&opts, &files).unwrap().processed_glyphs[0]
            .color_layers
            .is_none()
    );
    let files = vec![source("icon", "", "")];
    opts.color_selection = Some(&ColorSelection::All);
    assert!(
        prepare_svg_font(&opts, &files).unwrap().processed_glyphs[0]
            .color_layers
            .as_ref()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn color_pipeline_nested_curves_preserve_evenodd_coverage_after_reflection() {
    use kurbo::{BezPath, Point};
    let data =
        "M0 50C0 0 100 0 100 50C100 100 0 100 0 50Z M25 50C25 25 75 25 75 50C75 75 25 75 25 50Z";
    let files = vec![source(
        "icon",
        "",
        &format!(
            r#"<g transform="translate(100 0) scale(-1 1)"><path fill-rule="evenodd" d="{data}"/></g>"#
        ),
    )];
    let resolved = options(&files);
    let mut opts = svg_options_from_options(&resolved);
    opts.color_selection = Some(&ColorSelection::All);
    let prepared = prepare_svg_font(&opts, &files).unwrap();
    let outline = &prepared.processed_glyphs[0].color_layers.as_ref().unwrap()[0].outline;
    let original = BezPath::from_svg(data).unwrap();
    for x in (0..100).step_by(3) {
        for y in (0..100).step_by(3) {
            let point = Point::new(f64::from(x) + 0.31, f64::from(y) + 0.73);
            let mapped = Point::new(100.0 - point.x, 100.0 - point.y);
            assert_eq!(
                original.winding(point).abs() % 2 == 1,
                outline.winding(mapped) != 0
            );
        }
    }
}

fn assert_glyph(actual: &ProcessedGlyph, expected: &ProcessedGlyph) {
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.codepoint, expected.codepoint);
    assert_eq!(actual.width, expected.width);
    assert_eq!(actual.height, expected.height);
    assert_eq!(actual.path_data, expected.path_data);
    assert_eq!(actual.ttf_path, expected.ttf_path);
    assert_eq!(actual.ttf_path_hash, expected.ttf_path_hash);
    assert_eq!(actual.color_layers, expected.color_layers);
}

#[test]
fn color_pipeline_keeps_small_holes_next_to_curved_boundaries() {
    use kurbo::{BezPath, Point};
    let data = "M0 500C0 0 1000 0 1000 500C1000 1000 0 1000 0 500Z M370 139H380V141H370Z";
    for optimize in [false, true] {
        let files = vec![source(
            "icon",
            r#"viewBox="0 0 1000 1000""#,
            &format!(r#"<path fill-rule="evenodd" d="{data}"/>"#),
        )];
        let mut resolved = options(&files);
        resolved.font_height = Some(1000.0);
        resolved.optimize_output = Some(optimize);
        let mut opts = svg_options_from_options(&resolved);
        let mono = prepare_svg_font(&opts, &files).unwrap();
        opts.color_selection = Some(&ColorSelection::All);
        let color = prepare_svg_font(&opts, &files).unwrap();
        let glyph = &color.processed_glyphs[0];
        let outline = &glyph.color_layers.as_ref().unwrap()[0].outline;
        let original = BezPath::from_svg(data).unwrap();
        for (x, y, filled) in [
            (375.0, 140.0, false),
            (375.0, 143.0, true),
            (500.0, 500.0, true),
            (375.0, 120.0, false),
        ] {
            assert_eq!(original.winding(Point::new(x, y)).abs() % 2 == 1, filled);
            assert_eq!(outline.winding(Point::new(x, 1000.0 - y)) != 0, filled);
        }
        assert_eq!(glyph.ttf_path, mono.processed_glyphs[0].ttf_path);
    }
}

#[test]
fn color_pipeline_strokes_keep_fallback_and_only_extract_solid_fills() {
    for (body, count) in [
        (
            r#"<path d="M10 10L90 90" fill="none" stroke="blue" stroke-width="4"/>"#,
            0,
        ),
        (
            r#"<path d="M10 10H90V90H10Z" fill="red" stroke="blue" stroke-width="4"/>"#,
            1,
        ),
    ] {
        let files = vec![source("icon", "", body)];
        let resolved = options(&files);
        let mut opts = svg_options_from_options(&resolved);
        let mono = prepare_svg_font(&opts, &files).unwrap();
        opts.color_selection = Some(&ColorSelection::All);
        let color = prepare_svg_font(&opts, &files).unwrap();
        let glyph = &color.processed_glyphs[0];
        let layers = glyph.color_layers.as_ref().unwrap();
        assert_eq!(layers.len(), count);
        if count == 1 {
            assert_eq!(
                layers[0].paint,
                ResolvedLayerPaint::Solid {
                    red: 255,
                    green: 0,
                    blue: 0,
                    alpha: 1.0
                }
            );
        }
        assert!(!glyph.ttf_path.as_ref().unwrap().elements().is_empty());
        assert_eq!(glyph.ttf_path, mono.processed_glyphs[0].ttf_path);
        assert_eq!(glyph.ttf_path_hash, mono.processed_glyphs[0].ttf_path_hash);
    }
}

#[test]
fn color_pipeline_variant_layers_use_shared_advance_and_independent_source_scale() {
    let (family, codepoints) = resolved_family(
        vec![
            vec![variant_svg("small/a.svg", "a", 10, 20, 10)],
            vec![variant_svg("large/a.svg", "a", 30, 10, 15)],
        ],
        MissingGlyphBehavior::Blank,
        None,
        &["small", "large"],
    );
    for normalize in [false, true] {
        let mut resolved = options(&family.variants[0]);
        resolved.codepoints = codepoints.clone();
        resolved.font_height = Some(1000.0);
        resolved.descent = Some(100.0);
        resolved.normalize = normalize;
        resolved.center_horizontally = Some(true);
        resolved.center_vertically = Some(true);
        let mut opts = svg_options_from_options(&resolved);
        opts.color_selection = Some(&ColorSelection::All);
        let mut cache = VariantGlyphCache::default();
        let prepared = prepare_variant_svg_family_cached(&opts, &family, Some(&mut cache)).unwrap();
        let glyph = &prepared.glyphs[0];
        let advance = if normalize { 1000.0 } else { 1500.0 };
        assert!((glyph.advance_width - advance).abs() < 0.001);
        for (i, outline) in glyph.outlines.iter().enumerate() {
            let outline = outline.as_ref().unwrap();
            let bounds = outline.color_layers.as_ref().unwrap()[0]
                .outline
                .bounding_box();
            let (width, height) = match (normalize, i) {
                (_, 0) => (500.0, 1000.0),
                (true, _) => (500.0, 1000.0 / 3.0),
                (false, _) => (750.0, 500.0),
            };
            assert!((outline.width - advance).abs() < 0.001);
            assert!((bounds.width() - width).abs() < 0.001);
            assert!((bounds.height() - height).abs() < 0.001);
            assert!((bounds.center().x - advance / 2.0).abs() < 0.001);
            assert!((bounds.center().y - 400.0).abs() < 0.001);
        }
        let counts = (
            cache.process_count,
            cache.materialize_count,
            cache.parsed.iter().map(|c| c.parse_count).sum::<usize>(),
        );
        let reused = prepare_variant_svg_family_cached(&opts, &family, Some(&mut cache)).unwrap();
        assert_eq!(
            counts,
            (
                cache.process_count,
                cache.materialize_count,
                cache.parsed.iter().map(|c| c.parse_count).sum::<usize>()
            )
        );
        for (actual, expected) in reused.glyphs[0].outlines.iter().zip(&glyph.outlines) {
            assert_glyph(actual.as_ref().unwrap(), expected.as_ref().unwrap());
        }
    }
}

#[test]
fn color_pipeline_variant_logical_renames_reselect_unchanged_sources() {
    let mut variants = vec![
        vec![source("a", "", RECT), source("b", r#"fill="red""#, RECT)],
        vec![source("a", "", RECT), source("b", r#"fill="red""#, RECT)],
    ];
    let named = ColorSelection::Named(["a".into()].into());
    let mut cache = VariantGlyphCache::default();
    for renamed in [false, true] {
        if renamed {
            for files in &mut variants {
                files[0].glyph_name = "b".into();
                files[1].glyph_name = "a".into();
            }
        }
        let (family, codepoints) = resolved_family(
            variants.clone(),
            MissingGlyphBehavior::Blank,
            None,
            &["regular", "bold"],
        );
        let mut resolved = options(&variants[0]);
        resolved.codepoints = codepoints;
        let mut opts = svg_options_from_options(&resolved);
        opts.color_selection = Some(&named);
        let cached = prepare_variant_svg_family_cached(&opts, &family, Some(&mut cache)).unwrap();
        let fresh = prepare_variant_svg_family(&opts, &family).unwrap();
        for (actual, expected) in cached.glyphs.iter().zip(&fresh.glyphs) {
            for (actual, expected) in actual.outlines.iter().zip(&expected.outlines) {
                let actual = actual.as_ref().unwrap();
                assert_glyph(actual, expected.as_ref().unwrap());
                assert_eq!(actual.color_layers.is_some(), actual.name == "a");
                if let Some(layers) = &actual.color_layers {
                    assert_eq!(
                        layers[0].paint,
                        if renamed {
                            ResolvedLayerPaint::Solid {
                                red: 255,
                                green: 0,
                                blue: 0,
                                alpha: 1.0,
                            }
                        } else {
                            ResolvedLayerPaint::Foreground { alpha: 1.0 }
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn color_pipeline_resolves_authored_and_foreground_paint_in_order() {
    let files = vec![source(
        "icon",
        "",
        r##"
        <path d="M1 1H9V9Z" fill-opacity="0.1"/>
        <g color="red"><path d="M10 10H20V20Z" fill="currentColor"/>
          <path d="M20 20H30V30Z" color="blue" fill="currentColor"/></g>
        <style>.fixed { fill: #123456 }</style>
        <path d="M30 30H40V40Z" class="fixed"/>
        <defs><path id="shape" d="M40 40H50V50Z"/></defs>
        <use href="#shape" fill="currentColor"/>
        <path d="M60 60H70V70Z" fill="none"/>
    "##,
    )];
    let resolved = options(&files);
    let mut opts = svg_options_from_options(&resolved);
    opts.color_selection = Some(&ColorSelection::All);
    let prepared = prepare_svg_font(&opts, &files).unwrap();
    let paints: Vec<_> = prepared.processed_glyphs[0]
        .color_layers
        .as_ref()
        .unwrap()
        .iter()
        .map(|l| l.paint)
        .collect();
    assert_eq!(
        paints,
        [
            ResolvedLayerPaint::Foreground { alpha: 0.1 },
            ResolvedLayerPaint::Solid {
                red: 255,
                green: 0,
                blue: 0,
                alpha: 1.0
            },
            ResolvedLayerPaint::Solid {
                red: 0,
                green: 0,
                blue: 255,
                alpha: 1.0
            },
            ResolvedLayerPaint::Solid {
                red: 0x12,
                green: 0x34,
                blue: 0x56,
                alpha: 1.0
            },
            ResolvedLayerPaint::Foreground { alpha: 1.0 },
        ]
    );
}

#[test]
fn color_pipeline_root_fill_inline_color_and_marker_collision() {
    for (root, body, expected) in [
        (
            r#"fill="black""#,
            RECT,
            ResolvedLayerPaint::Solid {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 1.0,
            },
        ),
        (
            r#"color="red" style="color: blue; fill : currentColor""#,
            RECT,
            ResolvedLayerPaint::Solid {
                red: 0,
                green: 0,
                blue: 255,
                alpha: 1.0,
            },
        ),
        (
            r##"fill="#000001""##,
            RECT,
            ResolvedLayerPaint::Solid {
                red: 0,
                green: 0,
                blue: 1,
                alpha: 1.0,
            },
        ),
    ] {
        let files = vec![source("icon", root, body)];
        let resolved = options(&files);
        let mut opts = svg_options_from_options(&resolved);
        opts.color_selection = Some(&ColorSelection::All);
        let prepared = prepare_svg_font(&opts, &files).unwrap();
        assert_eq!(
            prepared.processed_glyphs[0].color_layers.as_ref().unwrap()[0].paint,
            expected
        );
    }
}

#[test]
fn color_pipeline_keeps_nonzero_coverage_and_evenodd_holes_independent() {
    use kurbo::Point;
    for optimize in [false, true] {
        for (rule, center_filled) in [("nonzero", true), ("evenodd", false)] {
            let files = vec![source(
                "icon",
                "",
                &format!(
                    r#"<path fill-rule="{rule}" d="M0 0H100V100H0Z M20 20H80V80H20Z"/><path fill="red" d="M40 40H60V60H40Z"/>"#
                ),
            )];
            let mut resolved = options(&files);
            resolved.optimize_output = Some(optimize);
            let mut opts = svg_options_from_options(&resolved);
            let mono = prepare_svg_font(&opts, &files).unwrap();
            opts.color_selection = Some(&ColorSelection::All);
            let color = prepare_svg_font(&opts, &files).unwrap();
            let glyph = &color.processed_glyphs[0];
            let layers = glyph.color_layers.as_ref().unwrap();
            assert_eq!(
                layers[0].outline.winding(Point::new(50.0, 50.0)) != 0,
                center_filled
            );
            assert_ne!(layers[1].outline.winding(Point::new(50.0, 50.0)), 0);
            assert_eq!(glyph.ttf_path, mono.processed_glyphs[0].ttf_path);
            assert_eq!(glyph.ttf_path_hash, mono.processed_glyphs[0].ttf_path_hash);
        }
    }
}

#[test]
fn color_pipeline_shares_viewbox_scale_center_and_rounding_with_fallback() {
    for optimize in [false, true] {
        let files = vec![source(
            "icon",
            r#"viewBox="10 20 50 100""#,
            r#"<g transform="translate(60 0) scale(-1 1)"><path d="M10 30H30V80H10Z"/></g>"#,
        )];
        let mut resolved = options(&files);
        resolved.font_height = Some(1000.0);
        resolved.descent = Some(100.0);
        resolved.center_horizontally = Some(true);
        resolved.center_vertically = Some(true);
        resolved.round = Some(10.0);
        resolved.optimize_output = Some(optimize);
        let mut opts = svg_options_from_options(&resolved);
        opts.color_selection = Some(&ColorSelection::All);
        let prepared = prepare_svg_font(&opts, &files).unwrap();
        let glyph = &prepared.processed_glyphs[0];
        let layer = &glyph.color_layers.as_ref().unwrap()[0];
        assert_eq!(Some(&layer.outline), glyph.ttf_path.as_ref());
        assert_eq!(Some(layer.outline_hash), glyph.ttf_path_hash);
        let bounds = layer.outline.bounding_box();
        assert_eq!(bounds.center().x, glyph.width / 2.0);
        assert_eq!(bounds.center().y, 400.0);
    }
}

#[test]
fn color_pipeline_incremental_selection_rename_and_content_reuse_match_fresh() {
    let mut files = vec![source("a", "", RECT), source("b", "", RECT)];
    let resolved = options(&files);
    let mut opts = svg_options_from_options(&resolved);
    let named = ColorSelection::Named(["a".into()].into());
    let mut cache = super::super::types::GlyphCache::default();
    for selection in [
        None,
        Some(&named),
        Some(&ColorSelection::All),
        Some(&named),
        None,
    ] {
        opts.color_selection = selection;
        for pass in 0..2 {
            let counts = (cache.parse_count, cache.process_count);
            let cached = prepare_svg_font_incremental(&opts, &files, &mut cache).unwrap();
            if pass == 1 {
                assert_eq!((cache.parse_count, cache.process_count), counts);
            }
            let fresh = prepare_svg_font(&opts, &files).unwrap();
            for (actual, expected) in cached.processed_glyphs.iter().zip(&fresh.processed_glyphs) {
                assert_glyph(actual, expected);
            }
        }
    }
    opts.color_selection = Some(&named);
    prepare_svg_font_incremental(&opts, &files, &mut cache).unwrap();
    // Rename logical identities without changing source paths or bytes.
    files[0].glyph_name = "b".into();
    files[1].glyph_name = "a".into();
    let cached = prepare_svg_font_incremental(&opts, &files, &mut cache).unwrap();
    let fresh = prepare_svg_font(&opts, &files).unwrap();
    for (actual, expected) in cached.processed_glyphs.iter().zip(&fresh.processed_glyphs) {
        assert_glyph(actual, expected);
    }
    assert!(cached.processed_glyphs[0].color_layers.is_none());
    assert!(cached.processed_glyphs[1].color_layers.is_some());
    files[1].contents = source("a", r#"fill="red""#, RECT).contents;
    cache.entries.remove(&files[1].path);
    cache.processed_entries.remove(&files[1].path);
    let cached = prepare_svg_font_incremental(&opts, &files, &mut cache).unwrap();
    let fresh = prepare_svg_font(&opts, &files).unwrap();
    assert_glyph(&cached.processed_glyphs[1], &fresh.processed_glyphs[1]);
}

#[test]
fn color_pipeline_variant_fallback_blank_and_paint_edits_match_fresh() {
    let mut variants = vec![
        vec![source("a", "", RECT), source("b", r#"fill="red""#, RECT)],
        vec![source("a", r#"fill="blue""#, RECT)],
    ];
    let resolved = options(&variants[0]);
    let mut opts = svg_options_from_options(&resolved);
    let named = ColorSelection::Named(["b".into()].into());
    let mut cache = VariantGlyphCache::default();
    for behavior in [MissingGlyphBehavior::Fallback, MissingGlyphBehavior::Blank] {
        for selection in [
            None,
            Some(&ColorSelection::All),
            Some(&named),
            None,
            Some(&named),
        ] {
            opts.color_selection = selection;
            let (family, _) =
                resolved_family(variants.clone(), behavior, Some(0), &["regular", "bold"]);
            let cached =
                prepare_variant_svg_family_cached(&opts, &family, Some(&mut cache)).unwrap();
            let fresh = prepare_variant_svg_family(&opts, &family).unwrap();
            for (actual, expected) in cached.glyphs.iter().zip(&fresh.glyphs) {
                assert_eq!(actual.advance_width, expected.advance_width);
                for (actual, expected) in actual.outlines.iter().zip(&expected.outlines) {
                    match (actual, expected) {
                        (Some(a), Some(e)) => assert_glyph(a, e),
                        (None, None) => {}
                        _ => panic!("blank state mismatch"),
                    }
                }
            }
        }
        variants[0][1].contents = source("b", r#"fill="green""#, RECT).contents;
    }
    let (family, _) = resolved_family(
        variants,
        MissingGlyphBehavior::Fallback,
        Some(0),
        &["regular", "bold"],
    );
    let cached = prepare_variant_svg_family_cached(&opts, &family, Some(&mut cache)).unwrap();
    let b = &cached.glyphs[1];
    assert_eq!(
        b.outlines[0].as_ref().unwrap().color_layers,
        b.outlines[1].as_ref().unwrap().color_layers
    );
    assert_eq!(
        b.outlines[0]
            .as_ref()
            .unwrap()
            .color_layers
            .as_ref()
            .unwrap()[0]
            .paint,
        ResolvedLayerPaint::Solid {
            red: 0,
            green: 128,
            blue: 0,
            alpha: 1.0
        }
    );
}
