use std::sync::Arc;

use kurbo::{BezPath, Point};
use write_fonts::read::tables::glyf::Glyph;
use write_fonts::read::tables::stat::AxisValue as ReadAxisValue;
use write_fonts::read::{FontRef, TableProvider};

use crate::input::resolve_generate_webfonts_options;
use crate::svg::types::{PreparedVariantFamily, ProcessedGlyph, ProcessedVariantGlyph};
use crate::{FontType, FontVariant, GenerateWebfontsOptions};

use super::*;

fn resolved_variants() -> ResolvedVariants {
    resolve_generate_webfonts_options(GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec![],
        types: Some(vec![FontType::Ttf]),
        variants: Some(vec![
            FontVariant {
                name: "Light".to_owned(),
                files: vec!["light.svg".to_owned()],
                weight: Some(300),
                default: None,
            },
            FontVariant {
                name: "Regular".to_owned(),
                files: vec!["regular.svg".to_owned()],
                weight: Some(400),
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
    .unwrap()
}

fn outline(name: &str, codepoint: u32, width: f64, shape_width: f64) -> ProcessedGlyph {
    let mut path = BezPath::new();
    path.move_to(Point::new(0.0, 0.0));
    path.line_to(Point::new(shape_width, 0.0));
    path.line_to(Point::new(shape_width, 10.0));
    path.line_to(Point::new(0.0, 10.0));
    path.close_path();
    ProcessedGlyph {
        codepoint,
        height: 10.0,
        index: 0,
        name: name.to_owned(),
        path_data: Arc::from(""),
        ttf_path: Some(Arc::new(path)),
        ttf_path_hash: None,
        width,
    }
}

fn family() -> PreparedVariantFamily {
    PreparedVariantFamily {
        ascent: 10.0,
        descent: 0.0,
        font_height: 1000.0,
        glyphs: vec![
            ProcessedVariantGlyph {
                name: "a".to_owned(),
                codepoint: 0xe001,
                advance_width: 30.0,
                outlines: vec![
                    Some(outline("a", 0xe001, 30.0, 10.0)),
                    Some(outline("a", 0xe001, 30.0, 20.0)),
                    Some(outline("a", 0xe001, 30.0, 30.0)),
                ]
                .into_boxed_slice(),
            },
            ProcessedVariantGlyph {
                name: "blank-default".to_owned(),
                codepoint: 0xe002,
                advance_width: 40.0,
                outlines: vec![
                    None,
                    None,
                    Some(outline("blank-default", 0xe002, 40.0, 10.0)),
                ]
                .into_boxed_slice(),
            },
        ],
    }
}

fn options() -> TtfOptions<'static> {
    TtfOptions {
        ascent: None,
        copyright: None,
        descent: None,
        description: None,
        font_height: None,
        font_name: "Variant Test",
        font_style: None,
        font_weight: None,
        ligature: true,
        manufacturer_url: None,
        ts: Some(0),
        version: None,
    }
}

#[test]
fn builds_deterministic_ordered_variant_glyph_store_and_default_cmap() {
    let variants = resolved_variants();
    let first = build_variant(options(), &family(), &variants).unwrap();
    let second = build_variant(options(), &family(), &variants).unwrap();
    assert_eq!(first.tables.ttf(), second.tables.ttf());
    assert_eq!(
        first.presentation_gids[0]
            .iter()
            .map(|gid| gid.to_u16())
            .collect::<Vec<_>>(),
        vec![3, 1, 4]
    );
    assert_eq!(
        first.presentation_gids[1]
            .iter()
            .map(|gid| gid.to_u16())
            .collect::<Vec<_>>(),
        vec![2, 2, 5]
    );

    let font = FontRef::new(first.tables.ttf()).expect("variant TTF should parse");
    assert_eq!(font.maxp().unwrap().num_glyphs(), 6);
    assert_eq!(
        font.cmap()
            .unwrap()
            .map_codepoint(0xe001_u32)
            .unwrap()
            .to_u32(),
        1
    );
    assert_eq!(
        font.cmap()
            .unwrap()
            .map_codepoint(0xe002_u32)
            .unwrap()
            .to_u32(),
        2
    );
    assert_eq!(font.cmap().unwrap().map_codepoint(0xe003_u32), None);
    let notdef = font
        .loca(None)
        .unwrap()
        .get_glyf(GlyphId::new(0), &font.glyf().unwrap())
        .unwrap();
    assert!(notdef.is_none());
    assert!(
        font.loca(None)
            .unwrap()
            .get_glyf(GlyphId::new(2), &font.glyf().unwrap())
            .unwrap()
            .is_none(),
        "blank default must target an explicit empty glyph"
    );
    let widths = (1..=5)
        .map(|gid| font.hmtx().unwrap().h_metrics()[gid].advance())
        .collect::<Vec<_>>();
    assert_eq!(widths, vec![30, 40, 30, 30, 40]);
    assert!(matches!(
        font.loca(None)
            .unwrap()
            .get_glyf(GlyphId::new(3), &font.glyf().unwrap())
            .unwrap(),
        Some(Glyph::Simple(_))
    ));
    assert!(font.gsub().is_err());
    assert!(font.table_data(Tag::new(b"gvar")).is_none());
}

#[test]
fn deduplicates_only_equal_compiled_outlines_and_clamped_advances() {
    let variants = resolved_variants();
    let same = outline("same", 0xe010, 20.0, 10.0);
    let different = outline("different", 0xe012, 20.0, 11.0);
    let family = PreparedVariantFamily {
        ascent: 10.0,
        descent: 0.0,
        font_height: 1000.0,
        glyphs: vec![
            ProcessedVariantGlyph {
                name: "same".to_owned(),
                codepoint: 0xe010,
                advance_width: 20.4,
                outlines: vec![Some(same.clone()), Some(same.clone()), Some(same.clone())]
                    .into_boxed_slice(),
            },
            ProcessedVariantGlyph {
                name: "same-outline-new-advance".to_owned(),
                codepoint: 0xe011,
                advance_width: 20.6,
                outlines: vec![Some(same.clone()), Some(same.clone()), Some(same.clone())]
                    .into_boxed_slice(),
            },
            ProcessedVariantGlyph {
                name: "different-outline".to_owned(),
                codepoint: 0xe012,
                advance_width: 20.4,
                outlines: vec![
                    Some(different.clone()),
                    Some(different.clone()),
                    Some(different),
                ]
                .into_boxed_slice(),
            },
            ProcessedVariantGlyph {
                name: "blank".to_owned(),
                codepoint: 0xe013,
                advance_width: 20.4,
                outlines: vec![None, None, None].into_boxed_slice(),
            },
        ],
    };
    let built = build_variant(options(), &family, &variants).unwrap();

    assert!(!built.tables.ttf().is_empty());
    assert_eq!(built.presentation_gids[0].as_ref(), &[GlyphId16::new(1); 3]);
    assert_eq!(built.presentation_gids[1].as_ref(), &[GlyphId16::new(2); 3]);
    assert_eq!(built.presentation_gids[2].as_ref(), &[GlyphId16::new(3); 3]);
    assert_eq!(built.presentation_gids[3].as_ref(), &[GlyphId16::new(4); 3]);
    assert_eq!(
        FontRef::new(built.tables.ttf())
            .unwrap()
            .maxp()
            .unwrap()
            .num_glyphs(),
        5
    );
}

#[test]
fn keeps_default_gids_distinct_when_logical_variant_rows_diverge() {
    let variants = resolved_variants();
    let shared_default = outline("shared", 0xe020, 20.0, 20.0);
    let family = PreparedVariantFamily {
        ascent: 10.0,
        descent: 0.0,
        font_height: 1000.0,
        glyphs: vec![
            ProcessedVariantGlyph {
                name: "first".to_owned(),
                codepoint: 0xe020,
                advance_width: 20.0,
                outlines: vec![
                    Some(outline("first", 0xe020, 20.0, 10.0)),
                    Some(shared_default.clone()),
                    Some(outline("first", 0xe020, 20.0, 30.0)),
                ]
                .into_boxed_slice(),
            },
            ProcessedVariantGlyph {
                name: "second".to_owned(),
                codepoint: 0xe021,
                advance_width: 20.0,
                outlines: vec![
                    Some(outline("second", 0xe021, 20.0, 11.0)),
                    Some(shared_default),
                    Some(outline("second", 0xe021, 20.0, 31.0)),
                ]
                .into_boxed_slice(),
            },
        ],
    };
    let built = build_variant(options(), &family, &variants).unwrap();

    assert_eq!(built.presentation_gids[0][1], GlyphId16::new(1));
    assert_eq!(built.presentation_gids[1][1], GlyphId16::new(2));
}

#[test]
fn writes_fvar_stat_names_and_default_weight_metadata() {
    let variants = resolved_variants();
    let mut options = options();
    options.version = Some("1.2");
    let built = build_variant(options, &family(), &variants).unwrap();
    let font = FontRef::new(built.tables.ttf()).unwrap();
    let name = font.name().unwrap();
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
    assert_eq!(read_name(256), "Weight");
    assert_eq!(read_name(5), "Version 1.2");
    assert_eq!(
        read_name(6),
        variant_postscript_name("Variant Test", "Regular")
    );
    assert_eq!(
        (read_name(257), read_name(258), read_name(259)),
        ("Light".to_owned(), "Regular".to_owned(), "Bold".to_owned())
    );

    let fvar = font.fvar().unwrap();
    let arrays = fvar.axis_instance_arrays().unwrap();
    let axis = arrays.axes()[0];
    assert_eq!(axis.axis_tag(), Tag::new(b"wght"));
    assert_eq!(axis.axis_name_id(), NameId::new(256));
    assert_eq!(
        (
            axis.min_value().to_i32(),
            axis.default_value().to_i32(),
            axis.max_value().to_i32()
        ),
        (300, 400, 700)
    );
    let instances = arrays
        .instances()
        .iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.subfamily_name_id.to_u16())
            .collect::<Vec<_>>(),
        vec![257, 258, 259]
    );
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.coordinates[0].get().to_i32())
            .collect::<Vec<_>>(),
        vec![300, 400, 700]
    );

    let stat = font.stat().unwrap();
    let design_axis = stat.design_axes().unwrap()[0];
    assert_eq!(
        (
            design_axis.axis_tag(),
            design_axis.axis_name_id(),
            design_axis.axis_ordering()
        ),
        (Tag::new(b"wght"), NameId::new(256), 0)
    );
    assert_eq!(stat.elided_fallback_name_id(), Some(NameId::new(258)));
    let values = stat.offset_to_axis_values().unwrap().unwrap().axis_values();
    assert_eq!(values.len(), 3);
    for index in 0..values.len() {
        let value = values.get(index).unwrap();
        let ReadAxisValue::Format1(value) = value else {
            panic!("STAT values must use Format 1")
        };
        assert_eq!(value.value_name_id(), NameId::new(257 + index as u16));
        assert_eq!(value.value().to_i32(), [300, 400, 700][index]);
        assert_eq!(
            value
                .flags()
                .contains(AxisValueTableFlags::ELIDABLE_AXIS_VALUE_NAME),
            index == 1
        );
    }
    let os2 = font.os2().unwrap();
    assert_eq!(os2.us_weight_class(), 400);
    assert_eq!(os2.us_first_char_index(), 0xe001);
    assert_eq!(os2.us_last_char_index(), 0xe002);
}

#[test]
fn makes_deterministic_postscript_safe_variant_names() {
    let first = variant_postscript_name(
        "Fónt /[](){}<>% with a very long family name that must be truncated",
        "large/alt",
    );
    let collision = variant_postscript_name(
        "Fónt /[](){}<>% with a very long family name that must be truncated",
        "large%alt",
    );

    assert!(first.is_ascii());
    assert!(first.len() <= 63);
    assert!(!first.chars().any(|character| {
        character.is_ascii_whitespace()
            || matches!(
                character,
                '[' | ']' | '(' | ')' | '{' | '}' | '<' | '>' | '/' | '%'
            )
    }));
    assert_ne!(first, collision);
    assert_eq!(
        variant_postscript_name("", "💡"),
        format!("{:x}", md5::compute(b"\0\xf0\x9f\x92\xa1"))
    );
}

#[test]
fn rejects_physical_glyph_overflow_before_gid16_conversion() {
    let error = checked_gids(usize::from(u16::MAX), vec![vec![1]]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Variant font has too many physical glyphs."
    );
}

#[test]
fn rejects_invalid_variant_metadata_and_glyph_matrix() {
    let mut variants = resolved_variants();
    variants.default_index = variants.variants.len();
    let error = build_variant(options(), &family(), &variants)
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);

    let variants = resolved_variants();
    let mut family = family();
    family.glyphs[0].outlines = vec![None].into_boxed_slice();
    let error = build_variant(options(), &family, &variants).err().unwrap();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}
