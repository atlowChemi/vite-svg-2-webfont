use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use kurbo::{PathEl, Point};
use write_fonts::read::{FontRef, TableProvider};

use crate::input::LoadedSvgFile;
use crate::input::{finalize_generate_webfonts_options, resolve_generate_webfonts_options};
use crate::svg::{prepare_svg_font, svg_options_from_options};
use crate::test_helpers::icons_root;
use crate::{FontType, FormatOptions, GenerateWebfontsOptions, TtfFormatOptions};

use super::outlines::SIMPLIFY_TOLERANCE;
use super::outlines::{point_line_distance, quadratic_path_from_svg_path_data};
use super::{build, current_unix_timestamp, ttf_options_from_options};

mod proof_font;

fn generate_ttf_font_bytes(options: GenerateWebfontsOptions) -> Result<Vec<u8>, Error> {
    let mut resolved_options = resolve_generate_webfonts_options(options)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error.to_string()))?;
    let source_files = resolved_options
        .files
        .iter()
        .map(|path| {
            Ok(LoadedSvgFile {
                contents: std::fs::read_to_string(path)?.into(),
                glyph_name: std::path::Path::new(path)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default()
                    .to_owned(),
                path: path.clone(),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    finalize_generate_webfonts_options(&mut resolved_options, &source_files)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error.to_string()))?;
    let binary_options = svg_options_from_options(&resolved_options);
    if resolved_options.types == [FontType::Ttf] {
        assert!(binary_options.structure_path);
        assert!(!binary_options.serialize_path);
    }
    let binary_prepared = prepare_svg_font(&binary_options, &source_files)
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
    let mut direct_options = ttf_options_from_options(&resolved_options);
    direct_options.ts = Some(direct_options.ts.unwrap_or_else(current_unix_timestamp));
    let timestamp = direct_options.ts;
    let direct = build(direct_options, &binary_prepared.processed_glyphs, None)?;
    if !resolved_options.types.contains(&FontType::Svg) {
        resolved_options.types.push(FontType::Svg);
    }
    let prepared = prepare_svg_font(&svg_options_from_options(&resolved_options), &source_files)
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?;
    let mut via_string = prepared.processed_glyphs.clone();
    for glyph in &mut via_string {
        glyph.ttf_path = None;
        glyph.ttf_path_hash = None;
    }
    let mut string_options = ttf_options_from_options(&resolved_options);
    string_options.ts = timestamp;
    let via_string = build(string_options, &via_string, None)?;
    assert_eq!(direct.ttf(), via_string.ttf());
    Ok(direct.ttf().to_vec())
}

#[test]
fn generates_a_ttf_buffer_with_a_true_type_header() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = root
        .join("src/svg/fixtures/icons/cleanicons")
        .join("plus.svg");
    let result = generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![fixture.display().to_string()],
        html: Some(false),
        font_name: Some("cleanicons".to_string()),
        ligature: Some(false),
        start_codepoint: Some(0xE001),
        types: Some(vec![FontType::Ttf]),
        ..Default::default()
    })
    .expect("expected native ttf generation to succeed");

    assert_eq!(&result[..4], &[0x00, 0x01, 0x00, 0x00]);
    assert!(!result.is_empty());
}

#[test]
fn public_metadata_options_are_written_to_ttf_tables() {
    let fixture = icons_root().join("cleanicons/plus.svg");
    let cases = [
        (
            Some("ItAlIc"),
            "BoLd",
            " 2.5. ",
            "Italic",
            "metadata Italic",
            "Version 2.5",
            700,
        ),
        (
            None,
            "250",
            "Version 3.0",
            "Regular",
            "metadata",
            "Version 3.0",
            250,
        ),
        (None, "0", "  ", "Regular", "metadata", "Version 1.0", 400),
    ];

    for (style, weight, version, subfamily, full_name, expected_version, weight_class) in cases {
        let bytes = generate_ttf_font_bytes(GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_string(),
            files: vec![fixture.display().to_string()],
            font_name: Some("metadata".to_string()),
            font_style: style.map(str::to_string),
            font_weight: Some(weight.to_string()),
            format_options: Some(FormatOptions {
                ttf: Some(TtfFormatOptions {
                    copyright: Some("Copyright 2026".to_string()),
                    description: None,
                    ts: Some(1_700_000_000),
                    url: None,
                    version: Some(version.to_string()),
                }),
                ..Default::default()
            }),
            html: Some(false),
            ligature: Some(false),
            types: Some(vec![FontType::Ttf]),
            ..Default::default()
        })
        .expect("TTF metadata generation should succeed");
        let font = FontRef::new(&bytes).expect("readable TTF");
        let name = font.name().expect("name table");
        let string_data = name.string_data();
        let read_name = |id| {
            name.name_record()
                .iter()
                .find(|record| record.name_id().to_u16() == id)
                .unwrap()
                .string(string_data)
                .unwrap()
                .to_string()
        };

        assert_eq!(read_name(0), "Copyright 2026");
        assert_eq!(read_name(2), subfamily);
        assert_eq!(read_name(4), full_name);
        assert_eq!(read_name(5), expected_version);
        assert_eq!(
            font.os2().expect("OS/2 table").us_weight_class(),
            weight_class
        );
    }
}

#[test]
fn optimized_direct_ttf_matches_the_string_path_route() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/svg/fixtures/icons/cleanicons");
    generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: [
            "account.svg",
            "arrow-down.svg",
            "arrow-left.svg",
            "arrow-right.svg",
            "arrow-up.svg",
            "basket.svg",
            "close.svg",
            "minus.svg",
            "plus.svg",
            "search.svg",
        ]
        .into_iter()
        .map(|file| root.join(file).display().to_string())
        .collect(),
        html: Some(false),
        font_name: Some("optimized-direct".to_string()),
        optimize_output: Some(true),
        types: Some(vec![FontType::Ttf]),
        ..Default::default()
    })
    .expect("expected direct optimized TTF generation to match the string route");
}

#[test]
fn adds_gsub_ligatures_and_placeholder_glyphs_when_enabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = root
        .join("src/svg/fixtures/icons/cleanicons")
        .join("plus.svg");
    let result = generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![fixture.display().to_string()],
        html: Some(false),
        font_name: Some("plus".to_string()),
        ligature: Some(true),
        start_codepoint: Some(0xE001),
        ..Default::default()
    })
    .expect("expected native ttf ligature generation to succeed");
    let font = FontRef::new(&result).expect("expected a readable ttf font");

    assert!(font.gsub().is_ok(), "expected a GSUB table for ligatures");
    assert!(font.maxp().expect("expected maxp table").num_glyphs() > 2);
}

fn create_svg_copies(
    source: &std::path::Path,
    names: &[&str],
) -> (std::path::PathBuf, Vec<String>) {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tmp = std::env::temp_dir().join(format!("ttf-dedup-{}-{unique}", names.join("-")));
    std::fs::create_dir_all(&tmp).unwrap();
    let files = names
        .iter()
        .map(|name| {
            let dest = tmp.join(format!("{name}.svg"));
            std::fs::copy(source, &dest).unwrap();
            dest.display().to_string()
        })
        .collect();
    (tmp, files)
}

#[test]
fn deduplicates_pair_of_identical_glyphs_with_explicit_codepoints() {
    let icon = icons_root().join("cleanicons/plus.svg");
    let (tmp, copies) = create_svg_copies(&icon, &["plus-copy"]);

    let mut files = vec![icon.display().to_string()];
    files.extend(copies);

    let result = generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        codepoints: Some(std::collections::HashMap::from([
            ("plus".to_string(), 0xE001u32),
            ("plus-copy".to_string(), 0xE002u32),
        ])),
        dest: "artifacts".to_string(),
        files,
        html: Some(false),
        font_name: Some("dedup-test".to_string()),
        ligature: Some(false),
        start_codepoint: Some(0xE001),
        ..Default::default()
    })
    .expect("TTF generation should succeed");
    let _ = std::fs::remove_dir_all(&tmp);

    let font = FontRef::new(&result).expect("readable TTF");
    assert_eq!(
        font.maxp().unwrap().num_glyphs(),
        2,
        "1 .notdef + 1 deduped glyph"
    );

    let cmap = font.cmap().expect("cmap");
    let gid_1 = cmap.map_codepoint(0xE001u32);
    let gid_2 = cmap.map_codepoint(0xE002u32);
    assert!(gid_1.is_some(), "E001 should be in cmap");
    assert_eq!(gid_1, gid_2, "both codepoints should map to same glyph ID");
}

#[test]
fn deduplicates_pair_of_identical_glyphs_with_implicit_codepoints() {
    let icon = icons_root().join("cleanicons/plus.svg");
    let (tmp, copies) = create_svg_copies(&icon, &["plus-copy"]);

    let mut files = vec![icon.display().to_string()];
    files.extend(copies);

    let result = generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files,
        html: Some(false),
        font_name: Some("dedup-implicit".to_string()),
        ligature: Some(false),
        start_codepoint: Some(0xF101),
        ..Default::default()
    })
    .expect("TTF generation should succeed");
    let _ = std::fs::remove_dir_all(&tmp);

    let font = FontRef::new(&result).expect("readable TTF");
    assert_eq!(
        font.maxp().unwrap().num_glyphs(),
        2,
        "1 .notdef + 1 deduped glyph"
    );

    let cmap = font.cmap().expect("cmap");
    let gid_1 = cmap.map_codepoint(0xF101u32);
    let gid_2 = cmap.map_codepoint(0xF102u32);
    assert!(gid_1.is_some());
    assert_eq!(
        gid_1, gid_2,
        "auto-assigned codepoints should map to same glyph ID"
    );
}

#[test]
fn deduplicates_multiple_copies_into_single_glyph() {
    let icon = icons_root().join("cleanicons/plus.svg");
    let (tmp, copies) = create_svg_copies(&icon, &["copy-a", "copy-b", "copy-c"]);

    let mut files = vec![icon.display().to_string()];
    files.extend(copies);

    let result = generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        codepoints: Some(std::collections::HashMap::from([
            ("plus".to_string(), 0xE001u32),
            ("copy-a".to_string(), 0xE002u32),
            ("copy-c".to_string(), 0xE003u32),
        ])),
        dest: "artifacts".to_string(),
        files,
        html: Some(false),
        font_name: Some("dedup-multi".to_string()),
        ligature: Some(false),
        start_codepoint: Some(0xE001),
        ..Default::default()
    })
    .expect("TTF generation should succeed");
    let _ = std::fs::remove_dir_all(&tmp);

    let font = FontRef::new(&result).expect("readable TTF");
    assert_eq!(
        font.maxp().unwrap().num_glyphs(),
        2,
        "4 identical SVGs → 1 .notdef + 1 deduped glyph"
    );

    let cmap = font.cmap().expect("cmap");
    let gids: Vec<_> = (0xE001u32..=0xE004u32)
        .map(|cp| cmap.map_codepoint(cp))
        .collect();
    assert!(
        gids.iter().all(|g| g.is_some()),
        "all 4 codepoints should be in cmap"
    );
    assert!(
        gids.windows(2).all(|w| w[0] == w[1]),
        "all 4 codepoints should map to same glyph ID"
    );
}

#[test]
fn does_not_deduplicate_glyphs_with_different_paths() {
    let result = generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        codepoints: Some(std::collections::HashMap::from([
            ("plus".to_string(), 0xE001u32),
            ("minus".to_string(), 0xE002u32),
        ])),
        dest: "artifacts".to_string(),
        files: vec![
            icons_root()
                .join("cleanicons/plus.svg")
                .display()
                .to_string(),
            icons_root()
                .join("cleanicons/minus.svg")
                .display()
                .to_string(),
        ],
        html: Some(false),
        font_name: Some("no-dedup".to_string()),
        ligature: Some(false),
        start_codepoint: Some(0xE001),
        ..Default::default()
    })
    .expect("TTF generation should succeed");

    let font = FontRef::new(&result).expect("readable TTF");
    assert_eq!(
        font.maxp().unwrap().num_glyphs(),
        3,
        "1 .notdef + 2 unique glyphs"
    );

    let cmap = font.cmap().expect("cmap");
    assert_ne!(
        cmap.map_codepoint(0xE001u32),
        cmap.map_codepoint(0xE002u32),
        "different glyphs should have different glyph IDs"
    );
}

#[test]
fn point_line_distance_measures_perpendicular_offset() {
    let a = Point::new(0.0, 0.0);
    let b = Point::new(4.0, 0.0);
    // 3 units above the line y = 0.
    assert!((point_line_distance(Point::new(2.0, 3.0), a, b) - 3.0).abs() < 1e-9);
    // Directly on the line.
    assert!(point_line_distance(Point::new(2.0, 0.0), a, b) < 1e-9);
}

#[test]
fn point_line_distance_falls_back_to_endpoint_distance_when_segment_is_degenerate() {
    let a = Point::new(1.0, 1.0);
    // a ≈ b, so there is no line — distance collapses to |p - a| = sqrt(3² + 4²) = 5.
    assert!((point_line_distance(Point::new(4.0, 5.0), a, a) - 5.0).abs() < 1e-9);
}

#[test]
fn merges_runs_of_collinear_lines_into_a_single_segment() {
    // Three collinear horizontal segments followed by a turn.
    let path = quadratic_path_from_svg_path_data("M0,0 L10,0 L20,0 L30,0 L30,10").unwrap();
    let lines = path
        .elements()
        .iter()
        .filter(|el| matches!(el, PathEl::LineTo(_)))
        .count();
    assert_eq!(
        lines, 2,
        "the collinear run collapses to one line; the turn stays"
    );
    assert!(
        matches!(path.elements().last(), Some(PathEl::LineTo(p)) if (p.x - 30.0).abs() < 1e-9 && (p.y - 10.0).abs() < 1e-9),
        "the final endpoint must be preserved exactly",
    );
}

#[test]
fn collapses_a_near_straight_quadratic_to_a_line() {
    // Control point sits ~0.1 units off the chord — within tolerance.
    const { assert!(SIMPLIFY_TOLERANCE > 0.1) };
    let path = quadratic_path_from_svg_path_data("M0,0 Q5,0.1 10,0").unwrap();
    assert!(
        !path
            .elements()
            .iter()
            .any(|el| matches!(el, PathEl::QuadTo(..))),
        "a near-straight quadratic should be emitted as a line, not a curve",
    );
}

#[test]
fn preserves_a_genuinely_curved_quadratic() {
    // Control point sits 50 units off the chord — a real curve.
    let path = quadratic_path_from_svg_path_data("M0,0 Q5,50 10,0").unwrap();
    assert!(
        path.elements()
            .iter()
            .any(|el| matches!(el, PathEl::QuadTo(..))),
        "a genuinely curved quadratic must be kept",
    );
}
