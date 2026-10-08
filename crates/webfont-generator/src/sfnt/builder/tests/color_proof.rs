//! Test-only feasibility spike: preserve production layout, append two unmapped
//! layer outlines, and attach static COLR v1 paints to both selectable GIDs.
use super::*;
use write_fonts::tables::colr::{
    BaseGlyphList, BaseGlyphPaint, Colr, LayerList, Paint, PaintColrLayers, PaintGlyph, PaintSolid,
};
use write_fonts::tables::cpal::{ColorRecord, Cpal};

fn color_proof() -> SerializedFontTables {
    let source = build_proof_font();
    let font = FontRef::new(source.ttf()).unwrap();
    assert_eq!(font.hhea().unwrap().number_of_h_metrics(), 5);
    let short = font.head().unwrap().index_to_loc_format() == 0;
    let loca = table_bytes(&source, *b"loca");
    let offsets: Vec<usize> = if short {
        loca.chunks_exact(2)
            .map(|v| usize::from(u16::from_be_bytes(v.try_into().unwrap())) * 2)
            .collect()
    } else {
        loca.chunks_exact(4)
            .map(|v| u32::from_be_bytes(v.try_into().unwrap()) as usize)
            .collect()
    };
    let mut glyf = table_bytes(&source, *b"glyf").to_vec();
    let mut offsets = offsets;
    // The original selectable outlines remain intact as monochrome fallback.
    for gid in [1, 2] {
        let outline = glyf[offsets[gid]..offsets[gid + 1]].to_vec();
        glyf.extend(outline);
        offsets.push(glyf.len());
    }
    let mut raw: Vec<_> = source
        .tables()
        .iter()
        .map(|table| (table.tag, table.bytes.clone()))
        .collect();
    for (tag, bytes) in &mut raw {
        match &*tag {
            b"glyf" => *bytes = glyf.clone(),
            b"loca" => {
                *bytes = offsets
                    .iter()
                    .flat_map(|v| (*v as u32).to_be_bytes())
                    .collect()
            }
            b"head" => bytes[50..52].copy_from_slice(&1_i16.to_be_bytes()),
            b"maxp" => bytes[4..6].copy_from_slice(&7_u16.to_be_bytes()),
            b"hhea" => {
                bytes[14..16].copy_from_slice(&(-900_i16).to_be_bytes());
                bytes[34..36].copy_from_slice(&7_u16.to_be_bytes());
            }
            b"hmtx" => {
                // Keep each outline's left sidebearing: head.flags declares
                // xMin == lsb. Zero advance does not imply a zero bearing.
                for gid in [1, 2] {
                    let lsb = bytes[gid * 4 + 2..gid * 4 + 4].to_vec();
                    bytes.extend([0_u8; 2]);
                    bytes.extend(lsb);
                }
            }
            // No new public names or cmap mappings for the two auxiliary glyphs.
            b"post" => {
                bytes.truncate(32);
                bytes[0..4].copy_from_slice(&0x0003_0000_u32.to_be_bytes());
            }
            _ => {}
        }
    }
    let layer = |gid, palette, alpha| -> Paint {
        PaintGlyph::new(
            PaintSolid::new(palette, F2Dot14::from_f32(alpha)).into(),
            GlyphId16::new(gid),
        )
        .into()
    };
    let colr = Colr {
        base_glyph_list: Some(BaseGlyphList::new(
            2,
            vec![
                BaseGlyphPaint::new(GlyphId16::new(1), PaintColrLayers::new(2, 0).into()),
                BaseGlyphPaint::new(GlyphId16::new(2), PaintColrLayers::new(2, 2).into()),
            ],
        ))
        .into(),
        layer_list: Some(LayerList::new(
            4,
            vec![
                layer(5, 0, 1.0),
                layer(6, 0xffff, 0.25),
                layer(6, 1, 1.0),
                layer(5, 0xffff, 0.25),
            ],
        ))
        .into(),
        ..Default::default()
    };
    let cpal = Cpal::new(
        2,
        1,
        2,
        Some(vec![
            ColorRecord::new(0, 0, 255, 255),
            ColorRecord::new(255, 0, 0, 255),
        ]),
        vec![0],
    );
    raw.push((*b"COLR", write_fonts::dump_table(&colr).unwrap()));
    raw.push((*b"CPAL", write_fonts::dump_table(&cpal).unwrap()));
    SerializedFontTables::new(raw).unwrap()
}

fn assert_color_semantics(bytes: &[u8]) {
    use write_fonts::read::tables::colr::Paint as ReadPaint;
    let font = FontRef::new(bytes).unwrap();
    assert_eq!(font.maxp().unwrap().num_glyphs(), 7);
    assert_eq!(font.hmtx().unwrap().h_metrics().len(), 7);
    let metrics = font.hmtx().unwrap();
    assert_eq!(metrics.h_metrics()[5].advance(), 0);
    assert_eq!(metrics.h_metrics()[5].side_bearing(), 100);
    assert_eq!(metrics.h_metrics()[6].advance(), 0);
    assert_eq!(metrics.h_metrics()[6].side_bearing(), 700);
    for (gid, x_min, x_max) in [(1, 100, 300), (2, 700, 900), (5, 100, 300), (6, 700, 900)] {
        let glyf = font.glyf().unwrap();
        let glyph = font
            .loca(None)
            .unwrap()
            .get_glyf(GlyphId::new(gid), &glyf)
            .unwrap()
            .unwrap();
        let write_fonts::read::tables::glyf::Glyph::Simple(glyph) = glyph else {
            panic!("simple outline expected")
        };
        assert_eq!((glyph.x_min(), glyph.x_max()), (x_min, x_max));
        assert_eq!(glyph.points().count(), 4);
    }
    let colr = font.colr().unwrap();
    assert_eq!(colr.version(), 1);
    let bases = colr.base_glyph_list().unwrap().unwrap();
    let layers = colr.layer_list().unwrap().unwrap();
    assert_eq!(bases.base_glyph_paint_records().len(), 2);
    for (index, base) in bases.base_glyph_paint_records().iter().enumerate() {
        assert_eq!(base.glyph_id(), GlyphId16::new(index as u16 + 1));
        let ReadPaint::ColrLayers(root) = base.paint(bases.offset_data()).unwrap() else {
            panic!("expected ordered layers")
        };
        assert_eq!(root.num_layers(), 2);
        assert_eq!(root.first_layer_index(), index as u32 * 2);
    }
    for (index, (gid, palette, alpha)) in [
        (5, 0, 1.0),
        (6, 0xffff, 0.25),
        (6, 1, 1.0),
        (5, 0xffff, 0.25),
    ]
    .into_iter()
    .enumerate()
    {
        let ReadPaint::Glyph(glyph) = layers.paints().get(index).unwrap() else {
            panic!("expected layer glyph")
        };
        assert_eq!(glyph.glyph_id(), GlyphId16::new(gid));
        let ReadPaint::Solid(paint) = glyph.paint().unwrap() else {
            panic!("expected solid paint")
        };
        assert_eq!(paint.palette_index(), palette);
        assert_eq!(paint.alpha(), F2Dot14::from_f32(alpha));
    }
    let cpal = font.cpal().unwrap();
    assert_eq!(cpal.num_palettes(), 1);
    let colors = cpal.color_records_array().unwrap().unwrap();
    assert_eq!((colors[0].red(), colors[0].alpha()), (255, 255));
    assert_eq!((colors[1].blue(), colors[1].alpha()), (255, 255));
    assert_eq!(font.fvar().unwrap().axes().unwrap().len(), 1);
    assert_eq!(font.stat().unwrap().design_axis_count(), 1);
    assert_eq!(font.gsub().unwrap().version(), MajorMinor::VERSION_1_1);
}

#[test]
fn color_proof_combined_tables_and_containers() {
    let tables = color_proof();
    assert_color_semantics(tables.ttf());
    let mono = build_proof_font();
    for tag in [*b"fvar", *b"STAT", *b"GSUB", *b"cmap"] {
        assert_eq!(table_bytes(&tables, tag), table_bytes(&mono, tag));
    }
    let woff1 = crate::formats::woff1::tables_to_woff1(&tables, None).unwrap();
    for table in tables.tables() {
        assert_eq!(woff1_table(&woff1, table.tag), table.bytes);
    }
    let woff2 = crate::formats::woff2::tables_to_woff2(&tables, 11, None).unwrap();
    let decoded = woff::version2::decompress(&woff2).unwrap();
    assert_color_semantics(&decoded);
    let decoded_font = FontRef::new(&decoded).unwrap();
    for tag in [*b"COLR", *b"CPAL", *b"fvar", *b"STAT", *b"GSUB", *b"cmap"] {
        assert_eq!(woff1_table(&woff1, tag), table_bytes(&tables, tag));
        assert_eq!(
            decoded_font.table_data(Tag::new(&tag)).unwrap().as_bytes(),
            table_bytes(&tables, tag)
        );
    }
    // Export only when requested by a downstream fixture task. Engine tests
    // validate their own generated data and never read adapter-owned assets.
    if let Some(directory) = std::env::var_os("COLOR_PROOF_OUTPUT_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        for (name, bytes) in [
            ("color-rvrn.ttf", tables.ttf()),
            ("color-rvrn.woff", woff1.as_slice()),
            ("color-rvrn.woff2", woff2.as_slice()),
        ] {
            std::fs::write(directory.join(name), bytes).unwrap();
        }
    }
}

// A PaintColrLayers node holds at most 255 children. Nested nodes preserve
// source order while lifting that limit without extra selectable glyphs.
fn nested_layers(nodes: Vec<Paint>, layers: &mut Vec<Paint>) -> Paint {
    assert!(!nodes.is_empty());
    if nodes.len() <= usize::from(u8::MAX) {
        let start = u32::try_from(layers.len()).unwrap();
        let count = u8::try_from(nodes.len()).unwrap();
        layers.extend(nodes);
        PaintColrLayers::new(count, start).into()
    } else {
        let groups = nodes
            .chunks(usize::from(u8::MAX))
            .map(|chunk| nested_layers(chunk.to_vec(), layers))
            .collect();
        nested_layers(groups, layers)
    }
}

#[test]
fn color_proof_nested_layer_boundaries_round_trip_in_source_order() {
    use write_fonts::read::tables::colr::{Colr as ReadColr, Paint as ReadPaint};
    fn visit(paint: ReadPaint<'_>, colr: &ReadColr<'_>, alphas: &mut Vec<F2Dot14>) {
        match paint {
            ReadPaint::ColrLayers(root) => {
                let layers = colr.layer_list().unwrap().unwrap();
                for index in root.first_layer_index()
                    ..root.first_layer_index() + u32::from(root.num_layers())
                {
                    visit(layers.paints().get(index as usize).unwrap(), colr, alphas);
                }
            }
            ReadPaint::Glyph(glyph) => {
                assert_eq!(glyph.glyph_id(), GlyphId16::new(5));
                let ReadPaint::Solid(solid) = glyph.paint().unwrap() else {
                    panic!("solid expected")
                };
                alphas.push(solid.alpha());
            }
            _ => panic!("unexpected graph node"),
        }
    }
    for count in [1, 255, 256, 511, 65_026] {
        let expected: Vec<_> = (0..count)
            .map(|i| F2Dot14::from_f32((i % 16385) as f32 / 16384.0))
            .collect();
        let paints = expected
            .iter()
            .map(|alpha| {
                PaintGlyph::new(PaintSolid::new(0, *alpha).into(), GlyphId16::new(5)).into()
            })
            .collect();
        let mut layers = vec![];
        let root = nested_layers(paints, &mut layers);
        let colr = Colr {
            base_glyph_list: Some(BaseGlyphList::new(
                1,
                vec![BaseGlyphPaint::new(GlyphId16::new(1), root)],
            ))
            .into(),
            layer_list: Some(LayerList::new(u32::try_from(layers.len()).unwrap(), layers)).into(),
            ..Default::default()
        };
        let baseline = color_proof();
        let mut tables: Vec<_> = baseline
            .tables()
            .iter()
            .map(|t| (t.tag, t.bytes.clone()))
            .collect();
        tables.iter_mut().find(|t| t.0 == *b"COLR").unwrap().1 =
            write_fonts::dump_table(&colr).unwrap();
        let serialized = SerializedFontTables::new(tables).unwrap();
        let font = FontRef::new(serialized.ttf()).unwrap();
        let colr = font.colr().unwrap();
        let bases = colr.base_glyph_list().unwrap().unwrap();
        let root = bases.base_glyph_paint_records()[0]
            .paint(bases.offset_data())
            .unwrap();
        let mut actual = vec![];
        visit(root, &colr, &mut actual);
        assert_eq!(actual, expected, "layer count {count}");
    }
}

#[test]
fn color_proof_cpal_maximum_palette_and_checked_overflow() {
    fn palette(count: usize) -> Result<Cpal, std::num::TryFromIntError> {
        let count = u16::try_from(count)?;
        Ok(Cpal::new(
            count,
            1,
            count,
            Some(
                (0..count)
                    .map(|i| ColorRecord::new(i as u8, (i >> 8) as u8, 0, 255))
                    .collect(),
            ),
            vec![0],
        ))
    }
    let baseline = color_proof();
    let mut tables: Vec<_> = baseline
        .tables()
        .iter()
        .map(|t| (t.tag, t.bytes.clone()))
        .collect();
    tables.iter_mut().find(|t| t.0 == *b"CPAL").unwrap().1 =
        write_fonts::dump_table(&palette(65_535).unwrap()).unwrap();
    let serialized = SerializedFontTables::new(tables).unwrap();
    let font = FontRef::new(serialized.ttf()).unwrap();
    let cpal = font.cpal().unwrap();
    assert_eq!(cpal.num_palette_entries(), 65_535);
    let records = cpal.color_records_array().unwrap().unwrap();
    assert_eq!(records.len(), 65_535);
    assert_eq!(records[65_534].blue(), 254);
    assert!(palette(65_536).is_err());
    // 0xffff is reserved for foreground, never an actual CPAL array index.
}
