use std::io::Error;
#[cfg(test)]
use std::io::ErrorKind;

use crate::sfnt::{SerializedFontTables, Woff2TransformCache};

mod woff2;

#[cfg(test)]
pub(crate) fn ttf_to_woff2(ttf: &[u8], quality: u8) -> Result<Vec<u8>, Error> {
    ::woff::version2::compress(ttf, "", quality.min(11) as usize, true)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 compression failed"))
}

pub(crate) fn tables_to_woff2(
    tables: &SerializedFontTables,
    quality: u8,
    cache: Option<&mut Woff2TransformCache>,
) -> Result<Vec<u8>, Error> {
    woff2::encode(tables, quality, cache)
}

#[cfg(feature = "bench")]
pub(crate) struct PreparedWoff2(woff2::PreparedWoff2);

#[cfg(feature = "bench")]
pub(crate) fn prepare_woff2(
    tables: &SerializedFontTables,
    cache: &mut Woff2TransformCache,
) -> Result<PreparedWoff2, Error> {
    woff2::prepare(tables, Some(cache)).map(PreparedWoff2)
}

#[cfg(feature = "bench")]
pub(crate) fn compress_prepared_woff2(
    prepared: &PreparedWoff2,
    quality: u8,
) -> Result<usize, Error> {
    woff2::compress(&prepared.0, quality).map(|compressed| compressed.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sfnt::{TtfOptions, build};
    use crate::svg::types::ProcessedGlyph;
    use std::collections::BTreeMap;
    use std::path::Path;
    use write_fonts::read::tables::glyf::Glyph;
    use write_fonts::read::types::GlyphId;
    use write_fonts::read::{FontRef, TableProvider};

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
            .join("src/woff/fixtures")
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
        let mut cache = crate::sfnt::Woff2TransformCache::default();
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
            .join("src/woff/fixtures")
            .join(format!("{glyph_count}.internal-woff2-baseline"));
        if std::env::var_os("UPDATE_WOFF2_FIXTURES").is_some_and(|value| value != "0") {
            std::fs::write(&baseline_path, &encoded)
                .expect("internal WOFF2 baseline should be written");
        } else {
            let baseline =
                std::fs::read(&baseline_path).expect("internal WOFF2 baseline should exist");
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
}
