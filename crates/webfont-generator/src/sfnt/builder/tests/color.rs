use std::io::ErrorKind;
use std::sync::Arc;

use kurbo::{BezPath, PathEl, Point};
use write_fonts::read::tables::colr::{Colr, Paint};
use write_fonts::read::{FontRef, TableProvider};
use write_fonts::types::{F2Dot14, GlyphId, GlyphId16, Tag};

use crate::formats::woff2::tables_to_woff2;
use crate::input::{
    LoadedSvgFile, ResolvedGenerateWebfontsOptions, finalize_generate_webfonts_options,
    resolve_generate_webfonts_options_with_color,
};
use crate::pipeline::TtfGlyphCache;
use crate::sfnt::SerializedFontTables;
use crate::svg::types::ProcessedGlyph;
use crate::svg::{prepare_svg_font, svg_options_from_options};
use crate::types::color::{ColorSelection, ProcessedColorLayer};
use crate::{FontType, FontVariant, GenerateWebfontsOptions, prepare_variant_family};

use super::super::color::build_color;
use super::super::glyphs::{build_glyf_table, compile_and_dedup_glyphs, compute_glyph_metrics};
use super::super::types::CompiledGlyphOutline;
use super::super::{build, build_variant, ttf_options_from_options};

#[test]
fn color_layers_respect_post_custom_name_limit() {
    let (resolved, mut glyphs) = prepare(&[r#"<path fill="red" d="M10 10H90V90H10Z"/>"#]);
    let outline = Arc::clone(&glyphs[0].color_layers.as_ref().unwrap()[0].outline);
    let outline_hash = glyphs[0].color_layers.as_ref().unwrap()[0].outline_hash;
    let paint = glyphs[0].color_layers.as_ref().unwrap()[0].paint;
    for (layer_count, name, succeeds) in [
        (65_277, "icon0", true),       // Exactly 65,278 unique custom names.
        (65_278, "icon0", false),      // One too many, but below the glyph limit.
        (65_278, "colr.layer2", true), // Duplicate names consume one index.
        (65_278, "A", true),           // Standard names consume no custom indices.
    ] {
        glyphs[0].name = name.into();
        glyphs[0].color_layers = Some(
            (0..layer_count)
                .map(|_| ProcessedColorLayer {
                    outline: Arc::clone(&outline),
                    outline_hash,
                    paint,
                })
                .collect(),
        );
        let result = build(ttf_options_from_options(&resolved), &glyphs, None);
        if succeeds {
            let tables = result.unwrap();
            let font = FontRef::new(tables.ttf()).unwrap();
            assert_eq!(
                usize::from(font.maxp().unwrap().num_glyphs()),
                layer_count + 2
            );
            assert_eq!(
                usize::from(font.post().unwrap().num_glyphs().unwrap()),
                layer_count + 2
            );
        } else {
            let error = result.err().unwrap();
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            assert_eq!(
                error.to_string(),
                "Font exceeds the 65278 unique custom glyph-name limit in the post table."
            );
        }
    }
}

#[test]
fn color_allocation_validates_initial_glyph_count() {
    // Include .notdef in the limit even when there are no selectable glyphs.
    let empty = build_color(&[], usize::from(u16::MAX) - 1).unwrap();
    assert!(empty.layers.is_empty());
    assert!(empty.tables.is_empty());
    for (placeholders, message) in [
        (
            usize::from(u16::MAX),
            "Font exceeds the 65,535-glyph limit.",
        ),
        (usize::MAX, "Font glyph count overflow."),
    ] {
        let error = build_color(&[], placeholders).err().unwrap();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn explicit_no_color_selection_matches_omitted_selection() {
    let files = vec![LoadedSvgFile {
        path: "icon.svg".into(), glyph_name: "icon".into(),
        contents: r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><path fill="red" d="M10 10H90V90H10Z"/></svg>"#.into(),
    }];
    let generate = |selection| {
        let mut options = resolve_generate_webfonts_options_with_color(
            GenerateWebfontsOptions {
                dest: "unused".into(),
                files: vec!["icon.svg".into()],
                types: Some(vec![FontType::Ttf]),
                write_files: Some(false),
                ..Default::default()
            },
            selection,
        )
        .unwrap();
        finalize_generate_webfonts_options(&mut options, &files).unwrap();
        let prepared = prepare_svg_font(&svg_options_from_options(&options), &files).unwrap();
        assert!(prepared.processed_glyphs[0].color_layers.is_none());
        let mut ttf = ttf_options_from_options(&options);
        ttf.ts = Some(0);
        build(ttf, &prepared.processed_glyphs, None).unwrap()
    };
    let omitted = generate(None);
    let disabled = generate(Some(ColorSelection::None));
    assert_eq!(omitted.ttf(), disabled.ttf());
    assert!(FontRef::new(disabled.ttf()).unwrap().colr().is_err());
}

#[test]
fn malformed_color_layer_reports_source_glyph() {
    let (options, mut glyphs) = prepare(&[r#"<path d="M10 10H90V90H10Z"/>"#]);
    // A cubic after a closed contour has no current point. The container is
    // valid, but this internal path cannot be converted to a TrueType outline.
    let malformed = BezPath::from_vec(vec![
        PathEl::MoveTo(Point::ZERO),
        PathEl::ClosePath,
        PathEl::CurveTo(
            Point::new(1.0, 2.0),
            Point::new(3.0, 4.0),
            Point::new(5.0, 6.0),
        ),
    ]);
    let paint = glyphs[0].color_layers.as_ref().unwrap()[0].paint;
    glyphs[0].color_layers = Some(Arc::from([ProcessedColorLayer {
        outline: Arc::new(malformed),
        outline_hash: 0,
        paint,
    }]));
    let error = build(ttf_options_from_options(&options), &glyphs, None)
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::Other);
    assert!(
        error
            .to_string()
            .contains("Failed to compile color layer for 'icon0'")
    );
    assert!(
        error
            .to_string()
            .contains("Encountered a cubic segment before a MoveTo")
    );
}

#[test]
fn rejected_auxiliary_glyph_reports_layer_name() {
    let (_, glyphs) = prepare(&[r#"<path d="M10 10H90V90H10Z"/>"#]);
    let (compiled, _) = compile_and_dedup_glyphs(&glyphs).unwrap();
    let mut color = build_color(&compiled, 0).unwrap();
    let CompiledGlyphOutline::Inline(outline) = &mut color.layers[0].outline else {
        panic!("expected an inline layer")
    };
    // Production layers have no instructions. Inject invalid internal data to
    // check that the table boundary preserves the layer context on rejection.
    outline.instructions = vec![0; usize::from(u16::MAX) + 1];
    let error = build_glyf_table(&compiled, &[], &color.layers)
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::Other);
    assert!(
        error
            .to_string()
            .contains("Failed to compile layer 'colr.layer2'")
    );
    assert!(error.to_string().contains("instructions len overflows"));
}

#[test]
fn glyph_metrics_preserve_empty_signed_and_average_values() {
    let empty = compute_glyph_metrics(std::iter::empty());
    assert_eq!(empty.bbox, (0, 0, 0, 0));
    assert_eq!(
        (
            empty.min_left_side_bearing,
            empty.min_right_side_bearing,
            empty.x_max_extent,
            empty.x_avg_char_width
        ),
        (0, 0, 0, 0)
    );
    let (_, glyphs) = prepare(&[r#"<path d="M10 10H90V90H10Z"/>"#]);
    let (mut compiled, _) = compile_and_dedup_glyphs(&glyphs).unwrap();
    let glyph = &mut compiled[0];
    glyph.advance_width = 0;
    glyph.left_side_bearing = -20;
    glyph.bbox.x_min = -20;
    glyph.bbox.x_max = -10;
    glyph.bbox.y_min = -15;
    glyph.bbox.y_max = -5;
    let negative = compute_glyph_metrics(compiled.iter());
    assert_eq!(negative.bbox, (-20, -15, 0, 0));
    assert_eq!(
        (
            negative.min_left_side_bearing,
            negative.min_right_side_bearing,
            negative.x_max_extent
        ),
        (-20, 10, -10)
    );
    assert_eq!(
        (
            negative.max_contours,
            negative.max_points,
            negative.x_avg_char_width
        ),
        (1, 4, 0)
    );
    let (mut advances, _) = compile_and_dedup_glyphs(&glyphs).unwrap();
    advances[0].advance_width = 11;
    let (mut other, _) = compile_and_dedup_glyphs(&glyphs).unwrap();
    other[0].advance_width = 12;
    let metrics = compute_glyph_metrics(compiled.iter().chain(&advances).chain(&other));
    assert_eq!(metrics.x_avg_char_width, 11); // Integer division, excluding zero advances.
    assert_eq!(metrics.advance_width_max, 12);
    let positive = compute_glyph_metrics(advances.iter());
    assert_eq!(positive.min_left_side_bearing, 10);
    assert_eq!(positive.x_max_extent, 90);
    advances[0].advance_width = u16::MAX;
    assert_eq!(
        compute_glyph_metrics(advances.iter()).x_avg_char_width,
        i16::MAX
    );
}

pub(super) fn production_color_font() -> SerializedFontTables {
    let mut options = resolve_generate_webfonts_options_with_color(
        GenerateWebfontsOptions {
            dest: "unused".into(),
            variants: Some(vec![
                FontVariant {
                    name: "Light".into(),
                    files: vec!["light.svg".into()],
                    weight: Some(300),
                    default: Some(true),
                },
                FontVariant {
                    name: "Bold".into(),
                    files: vec!["bold.svg".into()],
                    weight: Some(700),
                    default: None,
                },
            ]),
            types: Some(vec![FontType::Ttf, FontType::Woff, FontType::Woff2]),
            font_height: Some(1000.0),
            start_codepoint: Some(0xe001),
            ligature: Some(true),
            write_files: Some(false),
            ..Default::default()
        },
        Some(ColorSelection::All),
    )
    .unwrap();
    let sources = [("light.svg", "red", 100, 600), ("bold.svg", "blue", 700, 100)]
        .into_iter().map(|(path, color, fixed, foreground)| vec![LoadedSvgFile {
            path: path.into(), glyph_name: "ab".into(),
            contents: format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000"><path fill="{color}" d="M{fixed} 100h200v800h-200Z"/><path fill="currentColor" fill-opacity="0.25" d="M{foreground} 100h200v800h-200Z"/></svg>"#).into(),
        }]).collect();
    let (family, _, _) = prepare_variant_family(&mut options, sources).unwrap();
    let mut ttf = ttf_options_from_options(&options);
    ttf.ts = Some(0);
    build_variant(ttf, &family, options.variants.as_ref().unwrap())
        .unwrap()
        .tables
}
fn prepare(bodies: &[&str]) -> (ResolvedGenerateWebfontsOptions, Vec<ProcessedGlyph>) {
    let files: Vec<_> = bodies
        .iter()
        .enumerate()
        .map(|(i, body)| LoadedSvgFile {
            path: format!("icon{i}.svg"),
            glyph_name: format!("icon{i}"),
            contents: format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">{body}</svg>"#
            )
            .into(),
        })
        .collect();
    let mut resolved = resolve_generate_webfonts_options_with_color(
        GenerateWebfontsOptions {
            files: files.iter().map(|f| f.path.clone()).collect(),
            dest: "unused".into(),
            types: Some(vec![FontType::Ttf]),
            write_files: Some(false),
            ligature: Some(false),
            ..Default::default()
        },
        Some(ColorSelection::All),
    )
    .unwrap();
    finalize_generate_webfonts_options(&mut resolved, &files).unwrap();
    let opts = svg_options_from_options(&resolved);
    let glyphs = prepare_svg_font(&opts, &files).unwrap().processed_glyphs;
    (resolved, glyphs)
}

#[test]
fn color_identity_cache_and_auxiliary_counts() {
    let red = r#"<path fill="red" d="M10 10H90V90H10Z"/>"#;
    let blue = red.replace("red", "blue");
    let alpha = red.replace("fill=", "fill-opacity=\"0.5\" fill=");
    let (resolved, mut glyphs) = prepare(&[red, &blue, &alpha, red, red]);
    glyphs[4].color_layers = None;
    let mut cache = TtfGlyphCache::default();
    let generate = |glyphs: &[ProcessedGlyph], cache: Option<&mut TtfGlyphCache>| {
        let mut options = ttf_options_from_options(&resolved);
        options.ts = Some(0);
        build(options, glyphs, cache).unwrap()
    };
    let initial = generate(&glyphs, Some(&mut cache));
    assert_eq!(initial.ttf(), generate(&glyphs, None).ttf());
    let font = FontRef::new(initial.ttf()).unwrap();
    let ids: Vec<_> = glyphs
        .iter()
        .map(|g| font.cmap().unwrap().map_codepoint(g.codepoint).unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            GlyphId::new(1),
            GlyphId::new(2),
            GlyphId::new(3),
            GlyphId::new(1),
            GlyphId::new(4)
        ]
    );
    assert_eq!(font.maxp().unwrap().num_glyphs(), 8);
    assert_eq!(font.hmtx().unwrap().h_metrics().len(), 8);
    assert_eq!(font.post().unwrap().num_glyphs(), Some(8));
    assert_eq!(font.cpal().unwrap().num_palette_entries(), 2);
    let woff2 = tables_to_woff2(&initial, 11, None).unwrap();
    let decoded = ::woff::version2::decompress(&woff2).unwrap();
    let decoded_font = FontRef::new(&decoded).unwrap();
    for tag in [*b"COLR", *b"CPAL", *b"cmap", *b"maxp"] {
        let tag = Tag::new(&tag);
        assert_eq!(
            decoded_font.table_data(tag).unwrap().as_bytes(),
            font.table_data(tag).unwrap().as_bytes()
        );
    }
    assert_eq!(
        font.colr()
            .unwrap()
            .base_glyph_list()
            .unwrap()
            .unwrap()
            .base_glyph_paint_records()
            .len(),
        3
    );
    for metric in &font.hmtx().unwrap().h_metrics()[5..] {
        assert_eq!(metric.advance(), 0);
        assert_eq!(metric.side_bearing(), 10);
    }
    // Paint-only and layer-removal changes must not reuse stale table bytes.
    glyphs[0].color_layers = glyphs[1].color_layers.clone();
    assert_eq!(
        generate(&glyphs, Some(&mut cache)).ttf(),
        generate(&glyphs, None).ttf()
    );
    let (_, moved) = prepare(&[r#"<path fill="red" d="M20 10H80V90H20Z"/>"#]);
    glyphs[0].color_layers = moved[0].color_layers.clone();
    assert_eq!(
        generate(&glyphs, Some(&mut cache)).ttf(),
        generate(&glyphs, None).ttf()
    );
    for glyph in &mut glyphs {
        glyph.color_layers = None;
    }
    let mono = generate(&glyphs, Some(&mut cache));
    assert_eq!(mono.ttf(), generate(&glyphs, None).ttf());
    assert!(FontRef::new(mono.ttf()).unwrap().colr().is_err());
}

#[test]
fn color_format_validation_preserves_disabled_and_empty_selection() {
    let resolve = |types, selection| {
        resolve_generate_webfonts_options_with_color(
            GenerateWebfontsOptions {
                dest: "unused".into(),
                files: vec!["icon.svg".into()],
                types,
                ..Default::default()
            },
            selection,
        )
    };
    for types in [
        vec![FontType::Eot],
        vec![FontType::Svg],
        vec![FontType::Eot, FontType::Woff, FontType::Woff2],
    ] {
        for selection in [
            None,
            Some(ColorSelection::None),
            Some(ColorSelection::Named(Default::default())),
        ] {
            resolve(Some(types.clone()), selection).unwrap();
        }
        for selection in [
            ColorSelection::All,
            ColorSelection::Named(["icon0".into()].into()),
        ] {
            assert_eq!(
                resolve(Some(types.clone()), Some(selection))
                    .err()
                    .unwrap()
                    .kind(),
                ErrorKind::InvalidInput
            );
        }
    }
    for types in [
        vec![],
        vec![FontType::Ttf],
        vec![FontType::Woff, FontType::Woff2],
    ] {
        resolve(Some(types), Some(ColorSelection::All)).unwrap();
    }
    assert_eq!(
        resolve(None, Some(ColorSelection::All))
            .err()
            .unwrap()
            .kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn color_layer_order_is_part_of_selectable_identity() {
    let red = r#"<path fill="red" d="M10 10H90V90H10Z"/>"#;
    let blue = red.replace("red", "blue");
    let first = format!("{red}{blue}");
    let second = format!("{blue}{red}");
    let (resolved, glyphs) = prepare(&[&first, &second, &first]);
    assert_eq!(glyphs[0].ttf_path, glyphs[1].ttf_path);
    let tables = build(ttf_options_from_options(&resolved), &glyphs, None).unwrap();
    let font = FontRef::new(tables.ttf()).unwrap();
    let cmap = font.cmap().unwrap();
    assert_ne!(
        cmap.map_codepoint(glyphs[0].codepoint),
        cmap.map_codepoint(glyphs[1].codepoint)
    );
    assert_eq!(
        cmap.map_codepoint(glyphs[0].codepoint),
        cmap.map_codepoint(glyphs[2].codepoint)
    );
}

#[test]
fn color_foreground_empty_and_nested_layers() {
    for count in [0, 1, 255, 256, 511] {
        let body =
            r#"<path fill="currentColor" fill-opacity="0.25" d="M10 10H90V90H10Z"/>"#.repeat(count);
        let (resolved, glyphs) = prepare(&[&body]);
        let tables = build(ttf_options_from_options(&resolved), &glyphs, None).unwrap();
        let font = FontRef::new(tables.ttf()).unwrap();
        assert_eq!(font.maxp().unwrap().num_glyphs() as usize, count + 2);
        if count == 0 {
            assert!(font.colr().is_err());
            continue;
        }
        let cpal = font.cpal().unwrap();
        assert_eq!(cpal.num_palette_entries(), 1);
        assert_eq!(cpal.color_records_array().unwrap().unwrap()[0].alpha(), 255);
        let colr = font.colr().unwrap();
        let bases = colr.base_glyph_list().unwrap().unwrap();
        let root = bases.base_glyph_paint_records()[0]
            .paint(bases.offset_data())
            .unwrap();
        let mut ids = Vec::new();
        fn visit(paint: Paint<'_>, colr: &Colr<'_>, ids: &mut Vec<GlyphId16>) {
            match paint {
                Paint::ColrLayers(root) => {
                    let layers = colr.layer_list().unwrap().unwrap();
                    for index in root.first_layer_index()
                        ..root.first_layer_index() + u32::from(root.num_layers())
                    {
                        visit(layers.paints().get(index as usize).unwrap(), colr, ids);
                    }
                }
                Paint::Glyph(layer) => {
                    ids.push(layer.glyph_id());
                    let Paint::Solid(solid) = layer.paint().unwrap() else {
                        panic!()
                    };
                    assert_eq!(solid.palette_index(), 0xffff);
                    assert_eq!(solid.alpha(), F2Dot14::from_f32(0.25));
                }
                _ => panic!(),
            }
        }
        visit(root, &colr, &mut ids);
        assert_eq!(
            ids,
            (2..count + 2)
                .map(|gid| GlyphId16::new(gid as u16))
                .collect::<Vec<_>>()
        );
    }
    let (resolved, mut glyphs) = prepare(&[r#"<path d="M10 10H90V90H10Z"/>"#]);
    let layer = &glyphs[0].color_layers.as_ref().unwrap()[0];
    glyphs[0].color_layers = Some(
        (0..65_534)
            .map(|_| ProcessedColorLayer {
                outline: Arc::clone(&layer.outline),
                outline_hash: layer.outline_hash,
                paint: layer.paint,
            })
            .collect(),
    );
    let error = build(ttf_options_from_options(&resolved), &glyphs, None)
        .err()
        .unwrap();
    assert!(error.to_string().contains("65,535-glyph limit"));
}
