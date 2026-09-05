use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use flate2::read::ZlibDecoder;
use kurbo::{BezPath, Point};
use write_fonts::read::tables::gsub::{
    SingleSubst as ReadSingleSubst, SubstitutionLookup as ReadSubstitutionLookup,
};
use write_fonts::read::tables::layout::Condition as ReadCondition;
use write_fonts::read::{FontRef, TableProvider};
use write_fonts::types::{F2Dot14, Fixed, GlyphId, GlyphId16, MajorMinor, NameId, Tag};

use crate::input::resolve_generate_webfonts_options;
use crate::sfnt::SerializedFontTables;
use crate::svg::types::{PreparedVariantFamily, ProcessedGlyph, ProcessedVariantGlyph};
use crate::{FontType, FontVariant, GenerateWebfontsOptions};

use super::super::TtfOptions;

const WOFF_HEADER_SIZE: usize = 44;
const WOFF_TABLE_ENTRY_SIZE: usize = 20;

#[test]
fn proof_font_declares_discrete_weight_variation() {
    let tables = build_proof_font();
    let font = FontRef::new(tables.ttf()).expect("proof TTF should parse");

    assert_eq!(font.maxp().unwrap().num_glyphs(), 5);
    assert_eq!(
        font.cmap().unwrap().map_codepoint(0xe001_u32),
        Some(GlyphId::new(1))
    );
    assert!(font.table_data(Tag::new(b"gvar")).is_none());

    let fvar = font.fvar().expect("proof font should contain fvar");
    assert_eq!(fvar.version(), MajorMinor::VERSION_1_0);
    let axes = fvar.axes().unwrap();
    assert_eq!(axes.len(), 1);
    assert_eq!(axes[0].axis_tag(), Tag::new(b"wght"));
    assert_eq!(axes[0].min_value(), Fixed::from_i32(300));
    assert_eq!(axes[0].default_value(), Fixed::from_i32(300));
    assert_eq!(axes[0].max_value(), Fixed::from_i32(700));
    assert_eq!(axes[0].axis_name_id(), NameId::new(256));
    let stat = font.stat().expect("proof font should contain STAT");
    assert_eq!(stat.design_axis_count(), 1);
    let stat_axis = &stat.design_axes().unwrap()[0];
    assert_eq!(stat_axis.axis_tag(), Tag::new(b"wght"));
    assert_eq!(stat_axis.axis_name_id(), NameId::new(256));

    let h_metrics = font.hmtx().unwrap().h_metrics();
    assert_eq!(h_metrics[1].advance(), h_metrics[2].advance());

    let gsub = font.gsub().expect("proof font should contain GSUB");
    assert_eq!(gsub.version(), MajorMinor::VERSION_1_1);
    let feature = gsub.feature_list().unwrap().get(0).unwrap();
    assert_eq!(feature.tag, Tag::new(b"liga"));
    let liga_feature = feature;
    let feature = gsub.feature_list().unwrap().get(1).unwrap();
    assert_eq!(feature.tag, Tag::new(b"rvrn"));
    assert!(feature.lookup_list_indices().is_empty());

    let variations = gsub.feature_variations().unwrap().unwrap();
    assert_eq!(variations.feature_variation_record_count(), 1);
    let record = &variations.feature_variation_records()[0];
    let condition_set = record
        .condition_set(variations.offset_data())
        .unwrap()
        .unwrap();
    match condition_set.conditions().get(0).unwrap() {
        ReadCondition::Format1AxisRange(condition) => {
            assert_eq!(condition.axis_index(), 0);
            assert_eq!(condition.filter_range_min_value(), F2Dot14::from_f32(0.5));
            assert_eq!(condition.filter_range_max_value(), F2Dot14::ONE);
        }
        _ => panic!("expected an axis-range condition"),
    }

    let substitutions = record
        .feature_table_substitution(variations.offset_data())
        .unwrap()
        .unwrap();
    assert_eq!(substitutions.substitution_count(), 2);
    let alternate = substitutions.substitutions()[1]
        .alternate_feature(substitutions.offset_data())
        .unwrap();
    assert_eq!(alternate.lookup_list_indices()[0].get(), 0);

    let lookup = gsub.lookup_list().unwrap().lookups().get(0).unwrap();
    let ReadSubstitutionLookup::Single(lookup) = lookup else {
        panic!("expected a single-substitution lookup");
    };
    let ReadSingleSubst::Format2(single) = lookup.subtables().get(0).unwrap() else {
        panic!("expected SingleSubst format 2");
    };
    assert_eq!(
        single.coverage().unwrap().iter().collect::<Vec<_>>(),
        vec![GlyphId16::new(1)],
    );
    assert_eq!(single.substitute_glyph_ids()[0].get(), GlyphId16::new(2));
    let alternate_liga = substitutions.substitutions()[0]
        .alternate_feature(substitutions.offset_data())
        .unwrap();
    assert_eq!(
        ligature_target(&gsub, liga_feature.lookup_list_indices()[0].get()),
        GlyphId16::new(1)
    );
    assert_eq!(
        ligature_target(&gsub, alternate_liga.lookup_list_indices()[0].get()),
        GlyphId16::new(2)
    );
}

#[test]
fn proof_font_woff1_preserves_variable_tables() {
    let tables = build_proof_font();
    let woff = crate::formats::woff1::tables_to_woff1(&tables, None).unwrap();

    for tag in [*b"fvar", *b"STAT", *b"GSUB"] {
        assert_eq!(woff1_table(&woff, tag), table_bytes(&tables, tag));
    }
}

#[test]
fn proof_font_woff2_round_trips_variable_tables() {
    let tables = build_proof_font();
    let woff2 = crate::formats::woff2::tables_to_woff2(&tables, 11, None).unwrap();
    let decoded = woff::version2::decompress(&woff2).expect("proof WOFF2 should decode");
    let decoded = FontRef::new(&decoded).expect("decoded proof font should parse");

    for tag in [*b"fvar", *b"STAT", *b"GSUB"] {
        assert_eq!(
            decoded.table_data(Tag::new(&tag)).unwrap().as_bytes(),
            table_bytes(&tables, tag)
        );
    }
    let metrics = decoded.hmtx().unwrap().h_metrics();
    assert_eq!(metrics[1].advance(), metrics[2].advance());
}

#[test]
fn browser_proof_fixture_matches_test_builder() {
    let tables = build_proof_font();
    let actual = crate::formats::woff2::tables_to_woff2(&tables, 11, None).unwrap();
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/browser/fixtures/discrete-rvrn.woff2");

    if std::env::var_os("UPDATE_VARIABLE_PROOF_FIXTURE").is_some_and(|value| value != "0") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
    }

    assert_eq!(
        actual,
        std::fs::read(&path).expect("browser proof fixture should exist"),
        "browser proof fixture changed; inspect it and rerun with UPDATE_VARIABLE_PROOF_FIXTURE=1 to accept it",
    );
}

fn build_proof_font() -> SerializedFontTables {
    let variants = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec![],
        types: Some(vec![FontType::Ttf]),
        variants: Some(vec![
            FontVariant {
                name: "Light".to_owned(),
                files: vec!["light.svg".to_owned()],
                weight: Some(300),
                default: Some(true),
            },
            FontVariant {
                name: "Bold".to_owned(),
                files: vec!["bold.svg".to_owned()],
                weight: Some(700),
                default: None,
            },
        ]),
        write_files: Some(false),
        ..Default::default()
    })
    .unwrap()
    .variants
    .unwrap();
    let family = PreparedVariantFamily {
        ascent: 1000.0,
        descent: 0.0,
        font_height: 1000.0,
        glyphs: vec![ProcessedVariantGlyph {
            name: "ab".to_owned(),
            codepoint: 0xe001,
            advance_width: 1000.0,
            outlines: vec![proof_glyph(100.0, 300.0), proof_glyph(700.0, 900.0)].into_boxed_slice(),
        }],
    };
    super::super::build_variant(
        TtfOptions {
            ascent: Some(1000.0),
            copyright: None,
            descent: Some(0.0),
            description: None,
            font_height: Some(1000.0),
            font_name: "Discrete rvrn proof",
            font_style: None,
            font_weight: Some("300"),
            ligature: true,
            manufacturer_url: None,
            ts: Some(0),
            version: None,
        },
        &family,
        &variants,
    )
    .expect("proof variant font should build")
    .tables
}

fn proof_glyph(x_min: f64, x_max: f64) -> Option<ProcessedGlyph> {
    let mut path = BezPath::new();
    path.move_to(Point::new(x_min, 100.0));
    path.line_to(Point::new(x_max, 100.0));
    path.line_to(Point::new(x_max, 900.0));
    path.line_to(Point::new(x_min, 900.0));
    path.close_path();
    Some(ProcessedGlyph {
        codepoint: 0xe001,
        height: 1000.0,
        index: 0,
        name: "ab".to_owned(),
        path_data: Arc::from(""),
        ttf_path: Some(Arc::new(path)),
        ttf_path_hash: None,
        width: 1000.0,
    })
}

fn ligature_target(
    gsub: &write_fonts::read::tables::gsub::Gsub<'_>,
    lookup_index: u16,
) -> GlyphId16 {
    let lookup = gsub
        .lookup_list()
        .unwrap()
        .lookups()
        .get(usize::from(lookup_index))
        .unwrap();
    let ReadSubstitutionLookup::Ligature(lookup) = lookup else {
        panic!("expected a ligature lookup")
    };
    lookup
        .subtables()
        .get(0)
        .unwrap()
        .ligature_sets()
        .get(0)
        .unwrap()
        .ligatures()
        .get(0)
        .unwrap()
        .ligature_glyph()
}

fn table_bytes(tables: &crate::sfnt::SerializedFontTables, tag: [u8; 4]) -> &[u8] {
    &tables
        .tables()
        .iter()
        .find(|table| table.tag == tag)
        .unwrap()
        .bytes
}

fn woff1_table(woff: &[u8], wanted: [u8; 4]) -> Vec<u8> {
    let table_count = usize::from(u16::from_be_bytes(woff[12..14].try_into().unwrap()));
    for index in 0..table_count {
        let entry = WOFF_HEADER_SIZE + index * WOFF_TABLE_ENTRY_SIZE;
        if woff[entry..entry + 4] != wanted {
            continue;
        }
        let offset = u32::from_be_bytes(woff[entry + 4..entry + 8].try_into().unwrap()) as usize;
        let compressed_len =
            u32::from_be_bytes(woff[entry + 8..entry + 12].try_into().unwrap()) as usize;
        let original_len =
            u32::from_be_bytes(woff[entry + 12..entry + 16].try_into().unwrap()) as usize;
        let payload = &woff[offset..offset + compressed_len];
        if compressed_len == original_len {
            return payload.to_vec();
        }
        let mut decoded = Vec::with_capacity(original_len);
        ZlibDecoder::new(payload).read_to_end(&mut decoded).unwrap();
        return decoded;
    }
    panic!("missing WOFF1 table {wanted:?}");
}
