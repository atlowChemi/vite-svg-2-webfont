use super::compress::compress;
use super::glyf::{transform_cache_key, write_255_u16, write_triplet};
use super::prepare::prepare;
use super::serialize::{HEADER_SIZE, KNOWN_TAGS, assemble, write_base128, write_directory_entry};
use super::*;
use crate::sfnt::{SerializedTable, TtfOptions, build};
use crate::svg::types::ProcessedGlyph;
use crate::test_helpers::fixture_font_tables;
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};
use write_fonts::read::{
    FontRef, TableProvider,
    tables::{compute_checksum, glyf::Glyph},
    types::GlyphId,
};

const WOFF2_QUALITIES: std::ops::RangeInclusive<u8> = 0..=11;
type GlyphOutline = (i16, i16, i16, i16, Vec<u16>, Vec<(i16, i16, bool)>);

#[test]
fn woff2_acceptance_1_glyph() {
    assert_woff2_baseline(1);
}

#[test]
fn woff2_acceptance_100_glyphs() {
    assert_woff2_baseline(100);
}

#[test]
fn woff2_acceptance_300_glyphs() {
    assert_woff2_baseline(300);
}

#[test]
fn woff2_acceptance_600_glyphs() {
    assert_woff2_baseline(600);
}

#[test]
fn internal_woff2_acceptance_1_glyph() {
    assert_internal_woff2(1);
}

#[test]
fn internal_woff2_acceptance_100_glyphs() {
    assert_internal_woff2(100);
}

#[test]
fn internal_woff2_acceptance_300_glyphs() {
    assert_internal_woff2(300);
}

#[test]
fn internal_woff2_acceptance_600_glyphs() {
    assert_internal_woff2(600);
}

#[test]
fn internal_woff2_is_deterministic_for_curves() {
    let tables = font_tables(vec![glyph(
        0,
        "curve",
        "M0,0 C4,16 12,-8 16,16 Z",
        16.0,
        16.0,
    )]);
    for quality in WOFF2_QUALITIES {
        let output = tables_to_woff2(&tables, quality, None).unwrap();
        assert_eq!(output, tables_to_woff2(&tables, quality, None).unwrap());
        let decoded = ::woff::version2::decompress(&output).unwrap();
        assert_semantically_equal(tables.ttf(), &decoded);
    }
}

#[test]
fn woff2_round_trips_an_empty_glyph() {
    assert_woff2_roundtrip(&font_tables(vec![
        glyph(0, "empty", "", 16.0, 16.0),
        glyph(1, "visible", "M0,0 L16,0 L0,16 Z", 16.0, 16.0),
    ]));
}

#[test]
fn woff2_round_trips_duplicate_glyphs() {
    let tables = font_tables(vec![
        glyph(0, "original", "M0,0 L16,0 L0,16 Z", 16.0, 16.0),
        glyph(1, "duplicate", "M0,0 L16,0 L0,16 Z", 16.0, 16.0),
    ]);
    let font = FontRef::new(tables.ttf()).unwrap();
    let cmap = font.cmap().unwrap();
    assert_eq!(
        cmap.map_codepoint(0xe000_u32),
        cmap.map_codepoint(0xe001_u32)
    );
    assert_woff2_roundtrip(&tables);
}

#[test]
fn woff2_round_trips_ligatures() {
    let tables = font_tables(vec![glyph(
        0,
        "arrow-left",
        "M0,8 L16,0 L16,16 Z",
        16.0,
        16.0,
    )]);
    FontRef::new(tables.ttf())
        .unwrap()
        .gsub()
        .expect("GSUB should parse");
    assert_woff2_roundtrip(&tables);
}

fn assert_woff2_baseline(glyph_count: usize) {
    let tables = acceptance_font_tables(glyph_count);
    let source = tables.ttf();
    let baseline_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/formats/woff2/fixtures")
        .join(format!("{glyph_count}.woff2-baseline"));
    let outputs = WOFF2_QUALITIES
        .map(|quality| {
            let output = ttf_to_woff2(source, quality).expect("WOFF2 encoding should work");
            assert_eq!(
                output,
                ttf_to_woff2(source, quality).expect("repeated WOFF2 encoding should work"),
                "WOFF2 output changed between runs for {glyph_count} glyphs at quality {quality}",
            );
            let decoded = ::woff::version2::decompress(&output)
                .expect("reference WOFF2 decoding should work");
            assert_semantically_equal(source, &decoded);
            output
        })
        .collect::<Vec<_>>();
    let encoded = encode_baseline(&outputs);

    if std::env::var_os("UPDATE_WOFF2_FIXTURES").is_some_and(|value| value != "0") {
        std::fs::write(&baseline_path, &encoded).expect("WOFF2 baseline should be written");
    } else {
        let baseline = std::fs::read(&baseline_path).expect("WOFF2 baseline should exist");
        assert_eq!(
            encoded, baseline,
            "WOFF2 baseline changed for {glyph_count} glyphs; inspect it and rerun with UPDATE_WOFF2_FIXTURES=1 to accept it",
        );
    }
}

fn assert_internal_woff2(glyph_count: usize) {
    let tables = acceptance_font_tables(glyph_count);
    let mut cache = Woff2TransformCache::default();
    let outputs = WOFF2_QUALITIES
        .map(|quality| {
            let output = tables_to_woff2(&tables, quality, Some(&mut cache)).unwrap();
            assert_eq!(
                output,
                tables_to_woff2(&tables, quality, Some(&mut cache)).unwrap()
            );
            let decoded = ::woff::version2::decompress(&output)
                .expect("internal transformed WOFF2 should decode");
            assert_semantically_equal(tables.ttf(), &decoded);
            output
        })
        .collect::<Vec<_>>();
    assert_eq!(cache.compile_count, 1);

    let encoded = encode_baseline(&outputs);
    let baseline_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/formats/woff2/fixtures")
        .join(format!("{glyph_count}.internal-woff2-baseline"));
    if std::env::var_os("UPDATE_WOFF2_FIXTURES").is_some_and(|value| value != "0") {
        std::fs::write(&baseline_path, &encoded)
            .expect("internal WOFF2 baseline should be written");
    } else {
        let baseline = std::fs::read(&baseline_path).expect("internal WOFF2 baseline should exist");
        assert_eq!(
            encoded, baseline,
            "internal WOFF2 baseline changed for {glyph_count} glyphs; inspect it and rerun with UPDATE_WOFF2_FIXTURES=1 to accept it",
        );
    }
}

fn acceptance_font_tables(glyph_count: usize) -> SerializedFontTables {
    let glyphs = (0..glyph_count)
        .map(|index| {
            let empty = glyph_count > 1 && index % 97 == 0;
            let duplicate = index > 0 && index % 89 == 0;
            let shape = if duplicate { index - 1 } else { index };
            ProcessedGlyph {
                codepoint: 0xe000 + index as u32,
                height: 16.0 + (shape % 7) as f64,
                index,
                name: if index % 53 == 0 {
                    format!("renamed-glyph-{index}")
                } else {
                    format!("glyph-{index}")
                },
                path_data: (if empty {
                    String::new()
                } else {
                    let inset = shape % 5;
                    format!(
                        "M{inset},0 L{},{} L0,{} Z",
                        8 + shape % 11,
                        8 + shape % 13,
                        8 + shape % 17,
                    )
                })
                .into(),
                ttf_path: None,
                ttf_path_hash: None,
                width: 16.0 + (shape % 5) as f64,
            }
        })
        .collect::<Vec<_>>();
    font_tables(glyphs)
}

fn glyph(index: usize, name: &str, path_data: &str, width: f64, height: f64) -> ProcessedGlyph {
    ProcessedGlyph {
        codepoint: 0xe000 + index as u32,
        height,
        index,
        name: name.to_owned(),
        path_data: path_data.into(),
        ttf_path: None,
        ttf_path_hash: None,
        width,
    }
}

fn font_tables(glyphs: Vec<ProcessedGlyph>) -> SerializedFontTables {
    build(
        TtfOptions {
            ascent: None,
            copyright: None,
            descent: None,
            description: None,
            font_height: None,
            font_name: "WOFF2 Acceptance",
            font_style: None,
            font_weight: None,
            ligature: true,
            manufacturer_url: None,
            ts: Some(0),
            version: None,
        },
        &glyphs,
        None,
    )
    .expect("acceptance TTF should generate")
}

fn assert_woff2_roundtrip(tables: &SerializedFontTables) {
    let decoded = ::woff::version2::decompress(
        &ttf_to_woff2(tables.ttf(), 11).expect("WOFF2 encoding should work"),
    )
    .expect("reference WOFF2 decoding should work");
    assert_semantically_equal(tables.ttf(), &decoded);
}

fn encode_baseline(outputs: &[Vec<u8>]) -> Vec<u8> {
    let mut baseline = Vec::new();
    for output in outputs {
        baseline.extend_from_slice(&(output.len() as u32).to_be_bytes());
        baseline.extend_from_slice(output);
    }
    baseline
}

fn assert_semantically_equal(source: &[u8], decoded: &[u8]) {
    let source_font = FontRef::new(source).expect("source SFNT should parse");
    let decoded_font = FontRef::new(decoded).expect("decoded SFNT should parse");
    for font in [&source_font, &decoded_font] {
        font.cmap().expect("cmap should parse");
        font.glyf().expect("glyf should parse");
        font.hmtx().expect("hmtx should parse");
        font.name().expect("name should parse");
        font.head().expect("head should parse");
        font.hhea().expect("hhea should parse");
        font.maxp().expect("maxp should parse");
        font.os2().expect("OS/2 should parse");
    }
    assert_eq!(glyph_outlines(&source_font), glyph_outlines(&decoded_font));

    let mut source_tables = sfnt_tables(source);
    let mut decoded_tables = sfnt_tables(decoded);
    normalize_head(source_tables.get_mut(b"head").expect("source head table"));
    normalize_head(decoded_tables.get_mut(b"head").expect("decoded head table"));
    // The WOFF2 glyf transform may canonicalize equivalent point encodings and loca offsets.
    source_tables.remove(b"glyf");
    source_tables.remove(b"loca");
    decoded_tables.remove(b"glyf");
    decoded_tables.remove(b"loca");
    assert_eq!(source_tables, decoded_tables, "decoded SFNT tables changed");
}

fn glyph_outlines(font: &FontRef<'_>) -> Vec<Option<GlyphOutline>> {
    let glyf = font.glyf().unwrap();
    let loca = font.loca(None).unwrap();
    (0..font.maxp().unwrap().num_glyphs())
        .map(
            |id| match loca.get_glyf(GlyphId::new(id.into()), &glyf).unwrap() {
                None => None,
                Some(Glyph::Simple(glyph)) => Some((
                    glyph.x_min(),
                    glyph.y_min(),
                    glyph.x_max(),
                    glyph.y_max(),
                    glyph
                        .end_pts_of_contours()
                        .iter()
                        .map(|value| value.get())
                        .collect(),
                    glyph
                        .points()
                        .map(|point| (point.x, point.y, point.on_curve))
                        .collect(),
                )),
                Some(Glyph::Composite(_)) => {
                    panic!("acceptance fixtures should use simple glyphs")
                }
            },
        )
        .collect()
}

fn sfnt_tables(sfnt: &[u8]) -> BTreeMap<[u8; 4], Vec<u8>> {
    let count = u16::from_be_bytes(sfnt[4..6].try_into().unwrap()) as usize;
    (0..count)
        .map(|index| {
            let entry = 12 + index * 16;
            let tag = sfnt[entry..entry + 4].try_into().unwrap();
            let offset =
                u32::from_be_bytes(sfnt[entry + 8..entry + 12].try_into().unwrap()) as usize;
            let length =
                u32::from_be_bytes(sfnt[entry + 12..entry + 16].try_into().unwrap()) as usize;
            (tag, sfnt[offset..offset + length].to_vec())
        })
        .collect()
}

fn normalize_head(head: &mut [u8]) {
    head[8..12].fill(0); // checksumAdjustment is container-dependent.
    let flags = u16::from_be_bytes(head[16..18].try_into().unwrap()) & !(1 << 11);
    head[16..18].copy_from_slice(&flags.to_be_bytes());
}

#[test]
fn writes_canonical_base128_values() {
    for (value, expected) in [
        (0, &[0][..]),
        (127, &[0x7f]),
        (128, &[0x81, 0]),
        (16_383, &[0xff, 0x7f]),
        (16_384, &[0x81, 0x80, 0]),
        (u32::MAX, &[0x8f, 0xff, 0xff, 0xff, 0x7f]),
    ] {
        let mut actual = Vec::new();
        write_base128(value, &mut actual);
        assert_eq!(actual, expected);
    }
}

#[test]
fn writes_canonical_255_u16_boundaries() {
    for (value, expected) in [
        (0, &[0][..]),
        (252, &[252]),
        (253, &[255, 0]),
        (505, &[255, 252]),
        (506, &[254, 0]),
        (761, &[254, 255]),
        (762, &[253, 2, 250]),
        (u16::MAX, &[253, 255, 255]),
    ] {
        let mut actual = Vec::new();
        write_255_u16(value, &mut actual);
        assert_eq!(actual, expected);
    }
}

#[test]
fn triplets_cover_google_branch_boundaries_signs_and_curve_flag() {
    for (on_curve, x, y, encoded_len) in [
        (true, 0, 1279, 1),
        (true, 0, 1280, 3),
        (true, 1279, 0, 1),
        (true, 1280, 0, 3),
        (true, 64, 64, 1),
        (true, 65, 65, 2),
        (true, 768, 768, 2),
        (true, 769, 769, 3),
        (true, 4095, 4095, 3),
        (true, 4096, 4096, 4),
        (false, -64, 64, 1),
        (false, 64, -64, 1),
    ] {
        let mut flags = Vec::new();
        let mut glyphs = Vec::new();
        write_triplet(on_curve, x, y, &mut flags, &mut glyphs);
        assert_eq!(glyphs.len(), encoded_len);
        assert_eq!(decode_triplet(flags[0], &glyphs), (on_curve, x, y));
    }
}

#[test]
fn writes_known_and_unknown_directory_entries() {
    for (tag, expected) in [
        (*b"glyf", &[10, 127, 64][..]),
        (*b"loca", &[11, 127, 0][..]),
    ] {
        let mut output = Vec::new();
        write_directory_entry(
            &mut output,
            &SerializedTable {
                tag,
                checksum: 0,
                bytes: vec![0; 127],
            },
            64,
            127,
            127,
        )
        .unwrap();
        assert_eq!(output, expected);
    }

    let mut output = Vec::new();
    write_directory_entry(
        &mut output,
        &SerializedTable {
            tag: *b"TEST",
            checksum: 0,
            bytes: vec![0; 128],
        },
        64,
        127,
        127,
    )
    .unwrap();
    assert_eq!(output, [63, b'T', b'E', b'S', b'T', 0x81, 0]);
}

#[test]
fn transform_cache_key_hashes_tag_version_and_full_bodies() {
    let body = b"body".as_slice();
    let key = transform_cache_key(*b"glyf", 0, &[body]);
    assert_ne!(key, transform_cache_key(*b"loca", 0, &[body]));
    assert_ne!(key, transform_cache_key(*b"glyf", 1, &[body]));
    assert_ne!(key, transform_cache_key(*b"glyf", 0, &[b"Body"]));

    let first = [0, 0, 0, 1, 0, 0, 0, 2];
    let second = [0, 0, 0, 2, 0, 0, 0, 1];
    assert_eq!(compute_checksum(&first), compute_checksum(&second));
    assert_ne!(
        transform_cache_key(*b"glyf", 0, &[&first]),
        transform_cache_key(*b"glyf", 0, &[&second])
    );
}

#[test]
fn transform_cache_hits_and_prunes_unused_entries() {
    let mut cache = Woff2TransformCache::default();
    cache.insert(1, cached_payload(1));
    cache.insert(2, cached_payload(2));
    assert_eq!(cache.transformed(&1), Some(cached_payload(1)));
    assert_eq!(cache.compile_count, 2);

    cache.retain(&HashSet::from([2]));
    assert_eq!(cache.transformed(&1), None);
    assert_eq!(cache.transformed(&2), Some(cached_payload(2)));
}

#[test]
fn cache_hits_invalidates_and_prunes() {
    let tables = fixture_font_tables();
    let mut cache = Woff2TransformCache::default();
    prepare(&tables, Some(&mut cache)).unwrap();
    assert_eq!(cache.compile_count, 1);

    cache.insert(99, cached_payload(99));
    prepare(&tables, Some(&mut cache)).unwrap();
    assert_eq!(cache.compile_count, 2);
    assert_eq!(cache.transformed(&99), None);

    let mut raw = raw_tables(&tables);
    raw.iter_mut()
        .find(|(tag, _)| tag == b"glyf")
        .unwrap()
        .1
        .push(0);
    let changed = SerializedFontTables::new(raw).unwrap();
    prepare(&changed, Some(&mut cache)).unwrap();
    assert_eq!(cache.compile_count, 3);
}

#[test]
fn rejects_malformed_required_tables() {
    let tables = fixture_font_tables();

    let empty = SerializedFontTables::new(Vec::new()).unwrap();
    assert_eq!(
        prepare(&empty, None).err().unwrap().to_string(),
        "WOFF2 requires tables"
    );

    let mut missing_pair = raw_tables(&tables);
    missing_pair.retain(|(tag, _)| tag != b"loca");
    assert!(prepare(&SerializedFontTables::new(missing_pair).unwrap(), None).is_err());

    let mut invalid_format = raw_tables(&tables);
    table_bytes_mut(&mut invalid_format, b"head")[50..52].copy_from_slice(&2_i16.to_be_bytes());
    assert!(prepare(&SerializedFontTables::new(invalid_format).unwrap(), None).is_err());

    let mut wrong_count = raw_tables(&tables);
    table_bytes_mut(&mut wrong_count, b"loca").truncate(2);
    assert!(prepare(&SerializedFontTables::new(wrong_count).unwrap(), None).is_err());

    let mut duplicate = raw_tables(&tables);
    let head = duplicate
        .iter()
        .find(|(tag, _)| tag == b"head")
        .unwrap()
        .1
        .clone();
    duplicate.push((*b"head", head));
    assert!(prepare(&SerializedFontTables::new(duplicate).unwrap(), None).is_err());

    let mut invalid_loca = single_glyph_tables(simple_glyph(0x31, [0, 0, 10, 10]));
    let glyf_len = table_bytes_mut(&mut invalid_loca, b"glyf").len() as u32;
    table_bytes_mut(&mut invalid_loca, b"loca")[4..8]
        .copy_from_slice(&(glyf_len + 1).to_be_bytes());
    let invalid_loca = SerializedFontTables::new(invalid_loca).unwrap();
    assert_eq!(
        prepare(&invalid_loca, None).err().unwrap().to_string(),
        "loca offsets must be ascending and within glyf"
    );

    let mut invalid_endpoints = vec![0, 2];
    invalid_endpoints.extend_from_slice(&[0; 8]);
    invalid_endpoints.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0x31]);
    let invalid_endpoints =
        SerializedFontTables::new(single_glyph_tables(invalid_endpoints)).unwrap();
    assert_eq!(
        prepare(&invalid_endpoints, None).err().unwrap().to_string(),
        "invalid simple glyph contour endpoints"
    );
}

#[test]
fn preserves_simple_glyph_overlap() {
    let tables =
        SerializedFontTables::new(single_glyph_tables(simple_glyph(0x71, [0, 0, 10, 10]))).unwrap();
    let output = encode(&tables, 11, None).unwrap();
    let decoded = ::woff::version2::decompress(&output).unwrap();
    let font = FontRef::new(&decoded).unwrap();
    let glyph = font
        .loca(None)
        .unwrap()
        .get_glyf(GlyphId::new(0), &font.glyf().unwrap())
        .unwrap()
        .unwrap();
    let Glyph::Simple(glyph) = glyph else {
        panic!("expected a simple glyph");
    };
    assert!(glyph.has_overlapping_contours());
}

#[test]
fn preserves_noncanonical_simple_glyph_bbox() {
    let bbox = [-1, -2, 11, 12];
    let tables = SerializedFontTables::new(single_glyph_tables(simple_glyph(0x31, bbox))).unwrap();
    let output = encode(&tables, 11, None).unwrap();
    let decoded = ::woff::version2::decompress(&output).unwrap();
    let font = FontRef::new(&decoded).unwrap();
    let glyph = font
        .loca(None)
        .unwrap()
        .get_glyf(GlyphId::new(0), &font.glyf().unwrap())
        .unwrap()
        .unwrap();
    let Glyph::Simple(glyph) = glyph else {
        panic!("expected a simple glyph");
    };
    assert_eq!(
        [glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max()],
        bbox
    );
}

#[test]
fn rejects_composites_and_nonempty_zero_contours() {
    for contour_count in [-1_i16, 0] {
        let tables = fixture_font_tables();
        let mut raw = raw_tables(&tables);
        let head = table_bytes_mut(&mut raw, b"head").clone();
        let loca = table_bytes_mut(&mut raw, b"loca").clone();
        let offsets = loca_offsets(&loca, i16::from_be_bytes(head[50..52].try_into().unwrap()));
        let start = offsets.windows(2).find(|pair| pair[0] != pair[1]).unwrap()[0];
        table_bytes_mut(&mut raw, b"glyf")[start..start + 2]
            .copy_from_slice(&contour_count.to_be_bytes());
        let malformed = SerializedFontTables::new(raw).unwrap();
        assert!(prepare(&malformed, None).is_err());
    }
}

#[test]
fn split_preparation_compression_and_assembly_matches_encode() {
    let tables = fixture_font_tables();
    let prepared = prepare(&tables, None).unwrap();
    let compressed = compress(&prepared, 11).unwrap();
    assert_eq!(
        assemble(&prepared, &compressed).unwrap(),
        encode(&tables, 11, None).unwrap()
    );
}

#[test]
fn woff2_reference_decodes_semantically() {
    let tables = fixture_font_tables();
    let output = encode(&tables, 11, None).unwrap();
    let decoded = ::woff::version2::decompress(&output).expect("transformed WOFF2 should decode");
    assert_same_semantics(tables.ttf(), &decoded);

    let entries = directory_entries(&output);
    let glyf = entries.iter().find(|entry| entry.0 == *b"glyf").unwrap();
    let loca = entries.iter().find(|entry| entry.0 == *b"loca").unwrap();
    assert_eq!(glyf.1, 0);
    assert!(glyf.3.unwrap() > 0);
    assert_eq!(loca.1, 0);
    assert_eq!(loca.3, Some(0));
    assert_eq!(
        entries.iter().position(|entry| entry.0 == *b"loca"),
        entries
            .iter()
            .position(|entry| entry.0 == *b"glyf")
            .map(|index| index + 1)
    );
}

#[test]
fn promotes_short_loca_when_normalization_overflows() {
    const GLYPH_COUNT: u16 = 6_554;
    const GLYPH_SIZE: usize = 18;

    let mut glyph = vec![0, 1];
    glyph.extend_from_slice(&[0; 8]);
    glyph.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0x31, 0]);
    assert_eq!(glyph.len(), GLYPH_SIZE);

    let tables = fixture_font_tables();
    let mut raw = raw_tables(&tables);
    table_bytes_mut(&mut raw, b"head")[50..52].copy_from_slice(&0_i16.to_be_bytes());
    table_bytes_mut(&mut raw, b"maxp")[4..6].copy_from_slice(&GLYPH_COUNT.to_be_bytes());
    *table_bytes_mut(&mut raw, b"glyf") = glyph.repeat(usize::from(GLYPH_COUNT));
    *table_bytes_mut(&mut raw, b"loca") = (0..=GLYPH_COUNT)
        .flat_map(|index| ((usize::from(index) * GLYPH_SIZE / 2) as u16).to_be_bytes())
        .collect();
    let tables = SerializedFontTables::new(raw).unwrap();

    let output = encode(&tables, 0, None).unwrap();
    let decoded = ::woff::version2::decompress(&output).expect("transformed WOFF2 should decode");
    let head = sfnt_table(&decoded, b"head").unwrap();
    let loca = sfnt_table(&decoded, b"loca").unwrap();
    assert_eq!(i16::from_be_bytes(head[50..52].try_into().unwrap()), 1);
    assert_eq!(loca.len(), (usize::from(GLYPH_COUNT) + 1) * 4);
}

#[test]
fn removes_dsig_and_marks_the_font_as_transformed() {
    let tables = fixture_font_tables();
    let mut raw_tables = tables
        .tables()
        .iter()
        .map(|table| (table.tag, table.bytes.clone()))
        .collect::<Vec<_>>();
    raw_tables.push((*b"DSIG", vec![0; 8]));
    let tables = SerializedFontTables::new(raw_tables).unwrap();

    let output = encode(&tables, 11, None).unwrap();
    assert_eq!(
        u16::from_be_bytes(output[12..14].try_into().unwrap()) as usize,
        tables.tables().len() - 1
    );
    let decoded = ::woff::version2::decompress(&output).unwrap();
    assert!(sfnt_table(&decoded, b"DSIG").is_none());
    let head = sfnt_table(&decoded, b"head").unwrap();
    assert_ne!(
        u16::from_be_bytes(head[16..18].try_into().unwrap()) & (1 << 11),
        0
    );
}

#[test]
fn generates_woff2_font_with_expected_header() {
    let tables = crate::test_helpers::fixture_font_tables();

    let result = ttf_to_woff2(tables.ttf(), 10).expect("woff2 generation should succeed");

    assert_eq!(&result[..4], b"wOF2");
}

fn assert_same_semantics(source: &[u8], decoded: &[u8]) {
    let source = FontRef::new(source).expect("source should parse");
    let decoded = FontRef::new(decoded).expect("decoded font should parse");
    assert_eq!(
        source.maxp().unwrap().num_glyphs(),
        decoded.maxp().unwrap().num_glyphs()
    );
    assert_eq!(
        source.cmap().unwrap().map_codepoint(0xe001_u32),
        decoded.cmap().unwrap().map_codepoint(0xe001_u32)
    );
    assert_eq!(
        source.hhea().unwrap().ascender(),
        decoded.hhea().unwrap().ascender()
    );
    assert_eq!(
        source.name().unwrap().offset_data().as_bytes(),
        decoded.name().unwrap().offset_data().as_bytes()
    );
}

fn read_base128(input: &[u8], offset: &mut usize) -> u32 {
    let mut value = 0_u32;
    loop {
        let byte = input[*offset];
        *offset += 1;
        value = (value << 7) | u32::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return value;
        }
    }
}

fn decode_triplet(flag: u8, input: &[u8]) -> (bool, i32, i32) {
    let on_curve = flag & 0x80 == 0;
    let flag = flag & 0x7f;
    let with_sign = |flag: u8, value: i32| if flag & 1 != 0 { value } else { -value };
    let (x, y) = if flag < 10 {
        (
            0,
            with_sign(flag, (i32::from(flag & 14) << 7) + i32::from(input[0])),
        )
    } else if flag < 20 {
        (
            with_sign(
                flag,
                (i32::from((flag - 10) & 14) << 7) + i32::from(input[0]),
            ),
            0,
        )
    } else if flag < 84 {
        let b0 = flag - 20;
        (
            with_sign(flag, 1 + i32::from(b0 & 0x30) + i32::from(input[0] >> 4)),
            with_sign(
                flag >> 1,
                1 + i32::from((b0 & 0x0c) << 2) + i32::from(input[0] & 0x0f),
            ),
        )
    } else if flag < 120 {
        let b0 = flag - 84;
        (
            with_sign(flag, 1 + i32::from(b0 / 12) * 256 + i32::from(input[0])),
            with_sign(
                flag >> 1,
                1 + i32::from((b0 % 12) >> 2) * 256 + i32::from(input[1]),
            ),
        )
    } else if flag < 124 {
        (
            with_sign(flag, i32::from(input[0]) * 16 + i32::from(input[1] >> 4)),
            with_sign(
                flag >> 1,
                i32::from(input[1] & 0x0f) * 256 + i32::from(input[2]),
            ),
        )
    } else {
        (
            with_sign(flag, i32::from(u16::from_be_bytes([input[0], input[1]]))),
            with_sign(
                flag >> 1,
                i32::from(u16::from_be_bytes([input[2], input[3]])),
            ),
        )
    };
    (on_curve, x, y)
}

fn raw_tables(tables: &SerializedFontTables) -> Vec<([u8; 4], Vec<u8>)> {
    tables
        .tables()
        .iter()
        .map(|table| (table.tag, table.bytes.clone()))
        .collect()
}

fn simple_glyph(first_flag: u8, bbox: [i16; 4]) -> Vec<u8> {
    let mut glyph = 1_i16.to_be_bytes().to_vec();
    glyph.extend(bbox.into_iter().flat_map(i16::to_be_bytes));
    glyph.extend_from_slice(&[0, 2, 0, 0, first_flag, 0x33, 0x27, 10, 10, 10]);
    glyph
}

fn single_glyph_tables(glyph: Vec<u8>) -> Vec<([u8; 4], Vec<u8>)> {
    let tables = fixture_font_tables();
    let mut raw = raw_tables(&tables);
    table_bytes_mut(&mut raw, b"head")[50..52].copy_from_slice(&1_i16.to_be_bytes());
    table_bytes_mut(&mut raw, b"maxp")[4..6].copy_from_slice(&1_u16.to_be_bytes());
    let glyph_len = glyph.len() as u32;
    *table_bytes_mut(&mut raw, b"glyf") = glyph;
    *table_bytes_mut(&mut raw, b"loca") = [0_u32.to_be_bytes(), glyph_len.to_be_bytes()].concat();
    raw
}

fn cached_payload(byte: u8) -> Woff2TransformPayload {
    Woff2TransformPayload {
        transformed: vec![byte],
        normalized_glyf_len: usize::from(byte),
        normalized_glyf_checksum: u32::from(byte),
        normalized_loca_format: 0,
        normalized_loca_len: usize::from(byte),
        normalized_loca_checksum: u32::from(byte),
    }
}

fn table_bytes_mut<'a>(tables: &'a mut [([u8; 4], Vec<u8>)], tag: &[u8; 4]) -> &'a mut Vec<u8> {
    &mut tables.iter_mut().find(|table| &table.0 == tag).unwrap().1
}

fn loca_offsets(loca: &[u8], format: i16) -> Vec<usize> {
    if format == 0 {
        loca.as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| usize::from(u16::from_be_bytes(*bytes)) * 2)
            .collect()
    } else {
        loca.as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_be_bytes(*bytes) as usize)
            .collect()
    }
}

fn directory_entries(output: &[u8]) -> Vec<([u8; 4], u8, u32, Option<u32>)> {
    let count = u16::from_be_bytes(output[12..14].try_into().unwrap()) as usize;
    let mut offset = HEADER_SIZE;
    (0..count)
        .map(|_| {
            let flags = output[offset];
            offset += 1;
            let index = usize::from(flags & 0x3f);
            let tag = if index == 63 {
                let tag = output[offset..offset + 4].try_into().unwrap();
                offset += 4;
                tag
            } else {
                KNOWN_TAGS[index]
            };
            let length = read_base128(output, &mut offset);
            let transformed_length =
                matches!(&tag, b"glyf" | b"loca").then(|| read_base128(output, &mut offset));
            (tag, flags >> 6, length, transformed_length)
        })
        .collect()
}

fn sfnt_table<'a>(sfnt: &'a [u8], wanted: &[u8; 4]) -> Option<&'a [u8]> {
    let count = u16::from_be_bytes(sfnt[4..6].try_into().ok()?) as usize;
    (0..count).find_map(|index| {
        let entry = 12 + index * 16;
        if &sfnt[entry..entry + 4] != wanted {
            return None;
        }
        let offset = u32::from_be_bytes(sfnt[entry + 8..entry + 12].try_into().ok()?) as usize;
        let length = u32::from_be_bytes(sfnt[entry + 12..entry + 16].try_into().ok()?) as usize;
        sfnt.get(offset..offset + length)
    })
}
