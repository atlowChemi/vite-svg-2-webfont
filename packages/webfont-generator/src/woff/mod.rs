use std::hash::Hasher;
use std::io::{Error, ErrorKind, Write};

use crate::sfnt::SerializedFontTables;
use crate::ttf::{Woff1PayloadCache, Woff2TransformCache};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use rustc_hash::FxHasher;

mod woff2;

const WOFF_HEADER_SIZE: usize = 44;
const WOFF_TABLE_ENTRY_SIZE: usize = 20;
const META_OFFSET_POS: usize = 24;
const META_LENGTH_POS: usize = 28;
const META_ORIG_LENGTH_POS: usize = 32;
const LENGTH_POS: usize = 8;
const WOFF_SIGNATURE: [u8; 4] = *b"wOFF";

pub(crate) fn tables_to_woff1(
    tables: &SerializedFontTables,
    metadata: Option<&str>,
) -> Result<Vec<u8>, Error> {
    let mut woff_buf = encode_woff1(tables, None)?;
    if let Some(metadata) = metadata {
        inject_woff_metadata(&mut woff_buf, metadata)?;
    }
    Ok(woff_buf)
}

pub(crate) fn tables_to_woff1_cached(
    tables: &SerializedFontTables,
    metadata: Option<&str>,
    cache: &mut Woff1PayloadCache,
) -> Result<Vec<u8>, Error> {
    let mut woff_buf = encode_woff1(tables, Some(cache))?;
    if let Some(metadata) = metadata {
        inject_woff_metadata(&mut woff_buf, metadata)?;
    }
    Ok(woff_buf)
}

#[cfg(test)]
pub(crate) fn ttf_to_woff2(ttf: &[u8], quality: u8) -> Result<Vec<u8>, Error> {
    ::woff::version2::compress(ttf, "", quality.min(11) as usize, true)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 compression failed"))
}

#[cfg(feature = "bench")]
pub(crate) fn tables_to_woff2_no_transform(
    tables: &SerializedFontTables,
    quality: u8,
) -> Result<Vec<u8>, Error> {
    woff2::encode(tables, quality)
}

pub(crate) fn tables_to_woff2_transformed(
    tables: &SerializedFontTables,
    quality: u8,
    cache: Option<&mut Woff2TransformCache>,
) -> Result<Vec<u8>, Error> {
    woff2::encode_transformed(tables, quality, cache)
}

#[cfg(feature = "bench")]
pub(crate) struct PreparedWoff2(woff2::PreparedWoff2);

#[cfg(feature = "bench")]
pub(crate) fn prepare_woff2_no_transform(
    tables: &SerializedFontTables,
    cache: &mut Woff2TransformCache,
) -> Result<PreparedWoff2, Error> {
    woff2::prepare(tables, Some(cache)).map(PreparedWoff2)
}

#[cfg(feature = "bench")]
pub(crate) fn prepare_woff2_transformed(
    tables: &SerializedFontTables,
    cache: &mut Woff2TransformCache,
) -> Result<PreparedWoff2, Error> {
    woff2::prepare_transformed(tables, Some(cache)).map(PreparedWoff2)
}

#[cfg(feature = "bench")]
pub(crate) fn compress_prepared_woff2(
    prepared: &PreparedWoff2,
    quality: u8,
) -> Result<usize, Error> {
    woff2::compress(&prepared.0, quality).map(|compressed| compressed.len())
}

fn inject_woff_metadata(woff: &mut Vec<u8>, metadata: &str) -> Result<(), Error> {
    if woff.len() < WOFF_HEADER_SIZE {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "WOFF buffer is too short to contain a valid header.",
        ));
    }

    let meta_raw = metadata.as_bytes();
    let meta_compressed = zlib_compress(meta_raw, Compression::best())?;

    align4(woff);

    let meta_offset = woff.len() as u32;
    let meta_length = meta_compressed.len() as u32;
    let meta_orig_length = meta_raw.len() as u32;

    woff.extend_from_slice(&meta_compressed);

    let total_length = woff.len() as u32;

    woff[LENGTH_POS..LENGTH_POS + 4].copy_from_slice(&total_length.to_be_bytes());
    woff[META_OFFSET_POS..META_OFFSET_POS + 4].copy_from_slice(&meta_offset.to_be_bytes());
    woff[META_LENGTH_POS..META_LENGTH_POS + 4].copy_from_slice(&meta_length.to_be_bytes());
    woff[META_ORIG_LENGTH_POS..META_ORIG_LENGTH_POS + 4]
        .copy_from_slice(&meta_orig_length.to_be_bytes());

    Ok(())
}

fn encode_woff1(
    tables: &SerializedFontTables,
    mut cache: Option<&mut Woff1PayloadCache>,
) -> Result<Vec<u8>, Error> {
    let table_count = tables.tables().len();
    let mut used_cache_keys = std::collections::HashSet::new();
    let payloads = tables
        .tables()
        .iter()
        .map(|table| -> Result<([u8; 4], u32, u32, Vec<u8>), Error> {
            let original = table.bytes.as_slice();
            let cache_key = woff1_payload_cache_key(table.tag, original);
            used_cache_keys.insert(cache_key);
            let payload = if let Some(cache) = cache.as_deref_mut()
                && let Some(payload) = cache.woff1_payload(&cache_key)
            {
                payload
            } else {
                let compressed = zlib_compress(original, Compression::best())?;
                let payload = if compressed.len() < original.len() {
                    compressed
                } else {
                    original.to_vec()
                };
                if let Some(cache) = cache.as_deref_mut() {
                    cache.insert_woff1_payload(cache_key, payload.clone());
                }
                payload
            };
            Ok((table.tag, table.checksum, original.len() as u32, payload))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    if let Some(cache) = cache {
        cache.retain_woff1_payloads(&used_cache_keys);
    }
    let mut entries = Vec::with_capacity(table_count);
    let mut table_data = Vec::new();
    let mut data_offset = WOFF_HEADER_SIZE + table_count * WOFF_TABLE_ENTRY_SIZE;

    for (tag, checksum, orig_length, payload) in payloads {
        align4(&mut table_data);
        data_offset = align4_len(data_offset);

        entries.push((
            tag,
            data_offset as u32,
            payload.len() as u32,
            orig_length,
            checksum,
        ));
        table_data.extend_from_slice(&payload);
        data_offset += payload.len();
    }
    entries.sort_unstable_by_key(|entry| entry.0);
    align4(&mut table_data);

    let total_length =
        align4_len(WOFF_HEADER_SIZE + table_count * WOFF_TABLE_ENTRY_SIZE) + table_data.len();
    let mut woff = Vec::with_capacity(total_length);
    woff.extend_from_slice(&WOFF_SIGNATURE);
    woff.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    write_u32_be(&mut woff, total_length as u32);
    write_u16_be(&mut woff, table_count as u16);
    write_u16_be(&mut woff, 0);
    write_u32_be(&mut woff, total_sfnt_size(tables));
    write_u16_be(&mut woff, 1);
    write_u16_be(&mut woff, 0);
    write_u32_be(&mut woff, 0);
    write_u32_be(&mut woff, 0);
    write_u32_be(&mut woff, 0);
    write_u32_be(&mut woff, 0);
    write_u32_be(&mut woff, 0);

    for (tag, offset, comp_length, orig_length, checksum) in entries {
        woff.extend_from_slice(&tag);
        write_u32_be(&mut woff, offset);
        write_u32_be(&mut woff, comp_length);
        write_u32_be(&mut woff, orig_length);
        write_u32_be(&mut woff, checksum);
    }

    align4(&mut woff);
    woff.extend_from_slice(&table_data);
    Ok(woff)
}

fn zlib_compress(bytes: &[u8], compression: Compression) -> Result<Vec<u8>, Error> {
    let mut encoder = ZlibEncoder::new(Vec::new(), compression);
    encoder.write_all(bytes)?;
    encoder.finish()
}

// ponytail: non-crypto FxHash over tag + full table body. The SFNT checksum is
// NOT safe as a key here — it is a plain u32 word-sum and collides for distinct
// real font-table bodies (observed on `post`/`glyf`), which would reuse a stale
// compressed payload. Hashing the bytes with FxHash keeps the speed win over
// md5 without that collision risk. In-process only, so hasher stability across
// versions is a non-concern.
fn woff1_payload_cache_key(tag: [u8; 4], bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(&tag);
    hasher.write(bytes);
    hasher.finish()
}

fn total_sfnt_size(tables: &SerializedFontTables) -> u32 {
    let table_bytes: usize = tables
        .tables()
        .iter()
        .map(|table| align4_len(table.bytes.len()))
        .sum();
    (12 + tables.tables().len() * 16 + table_bytes) as u32
}

fn align4(bytes: &mut Vec<u8>) {
    bytes.resize(align4_len(bytes.len()), 0);
}

fn align4_len(len: usize) -> usize {
    (len + 3) & !3
}

fn write_u16_be(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn write_u32_be(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::types::ProcessedGlyph;
    use crate::test_helpers::fixture_font_tables;
    use crate::ttf::{TtfOptions, generate_ttf_font_from_glyphs};
    use flate2::read::ZlibDecoder;
    use std::collections::BTreeMap;
    use std::io::Read;
    use std::path::Path;
    use write_fonts::read::tables::glyf::Glyph;
    use write_fonts::read::types::GlyphId;
    use write_fonts::read::{FontRef, TableProvider};

    const WOFF2_QUALITIES: std::ops::RangeInclusive<u8> = 0..=11;
    type GlyphOutline = (i16, i16, i16, i16, Vec<u16>, Vec<(i16, i16, bool)>);

    #[test]
    fn injects_metadata_into_woff_header() {
        let mut woff = vec![0u8; WOFF_HEADER_SIZE];

        inject_woff_metadata(&mut woff, "<metadata />").unwrap();

        assert!(woff.len() > WOFF_HEADER_SIZE);

        let total_length = u32::from_be_bytes(woff[LENGTH_POS..LENGTH_POS + 4].try_into().unwrap());
        let meta_offset = u32::from_be_bytes(
            woff[META_OFFSET_POS..META_OFFSET_POS + 4]
                .try_into()
                .unwrap(),
        );
        let meta_length = u32::from_be_bytes(
            woff[META_LENGTH_POS..META_LENGTH_POS + 4]
                .try_into()
                .unwrap(),
        );
        let meta_orig = u32::from_be_bytes(
            woff[META_ORIG_LENGTH_POS..META_ORIG_LENGTH_POS + 4]
                .try_into()
                .unwrap(),
        );

        assert_eq!(total_length, woff.len() as u32);
        assert_eq!(meta_offset, WOFF_HEADER_SIZE as u32);
        assert_eq!(meta_orig, 12);
        assert!(meta_length > 0);
    }

    #[test]
    fn rejects_buffer_too_short() {
        let mut woff = vec![0u8; 10];
        let err = inject_woff_metadata(&mut woff, "test").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn woff1_tables_round_trip_to_sfnt_tables() {
        let tables = fixture_font_tables();
        let woff = tables_to_woff1(&tables, None).expect("expected woff generation to succeed");

        assert_eq!(&woff[0..4], b"wOFF");
        assert_eq!(
            u16::from_be_bytes(woff[12..14].try_into().unwrap()) as usize,
            tables.tables().len()
        );

        for index in 0..tables.tables().len() {
            let entry_offset = WOFF_HEADER_SIZE + index * WOFF_TABLE_ENTRY_SIZE;
            let tag: [u8; 4] = woff[entry_offset..entry_offset + 4].try_into().unwrap();
            let table = tables
                .tables()
                .iter()
                .find(|table| table.tag == tag)
                .expect("expected WOFF table tag to exist in SFNT");
            let offset =
                u32::from_be_bytes(woff[entry_offset + 4..entry_offset + 8].try_into().unwrap())
                    as usize;
            let comp_len = u32::from_be_bytes(
                woff[entry_offset + 8..entry_offset + 12]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let orig_len = u32::from_be_bytes(
                woff[entry_offset + 12..entry_offset + 16]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let payload = &woff[offset..offset + comp_len];
            let decoded = if comp_len < orig_len {
                let mut decoded = Vec::new();
                ZlibDecoder::new(payload)
                    .read_to_end(&mut decoded)
                    .expect("expected table payload to decompress");
                decoded
            } else {
                payload.to_vec()
            };

            assert_eq!(decoded, table.bytes);
        }
    }

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
    fn internal_no_transform_woff2_acceptance_1_glyph() {
        assert_internal_no_transform_woff2(1);
    }

    #[test]
    fn internal_no_transform_woff2_acceptance_100_glyphs() {
        assert_internal_no_transform_woff2(100);
    }

    #[test]
    fn internal_no_transform_woff2_acceptance_300_glyphs() {
        assert_internal_no_transform_woff2(300);
    }

    #[test]
    fn internal_no_transform_woff2_acceptance_600_glyphs() {
        assert_internal_no_transform_woff2(600);
    }

    #[test]
    fn internal_transformed_woff2_acceptance_1_glyph() {
        assert_internal_transformed_woff2(1);
    }

    #[test]
    fn internal_transformed_woff2_acceptance_100_glyphs() {
        assert_internal_transformed_woff2(100);
    }

    #[test]
    fn internal_transformed_woff2_acceptance_300_glyphs() {
        assert_internal_transformed_woff2(300);
    }

    #[test]
    fn internal_transformed_woff2_acceptance_600_glyphs() {
        assert_internal_transformed_woff2(600);
    }

    #[test]
    fn internal_transformed_woff2_is_deterministic_for_curves() {
        let tables = font_tables(vec![glyph(
            0,
            "curve",
            "M0,0 C4,16 12,-8 16,16 Z",
            16.0,
            16.0,
        )]);
        for quality in WOFF2_QUALITIES {
            let output = tables_to_woff2_transformed(&tables, quality, None).unwrap();
            assert_eq!(
                output,
                tables_to_woff2_transformed(&tables, quality, None).unwrap()
            );
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

    fn assert_internal_no_transform_woff2(glyph_count: usize) {
        let tables = acceptance_font_tables(glyph_count);
        for quality in WOFF2_QUALITIES {
            let output = super::woff2::encode(&tables, quality).unwrap();
            assert_eq!(output, super::woff2::encode(&tables, quality).unwrap());
            let decoded = ::woff::version2::decompress(&output)
                .expect("internal no-transform WOFF2 should decode");
            assert_semantically_equal(tables.ttf(), &decoded);
        }
    }

    fn assert_internal_transformed_woff2(glyph_count: usize) {
        let tables = acceptance_font_tables(glyph_count);
        let mut cache = crate::ttf::Woff2TransformCache::default();
        for quality in WOFF2_QUALITIES {
            let output = tables_to_woff2_transformed(&tables, quality, Some(&mut cache)).unwrap();
            assert_eq!(
                output,
                tables_to_woff2_transformed(&tables, quality, Some(&mut cache)).unwrap()
            );
            let decoded = ::woff::version2::decompress(&output)
                .expect("internal transformed WOFF2 should decode");
            assert_semantically_equal(tables.ttf(), &decoded);
        }
        assert_eq!(cache.compile_count, 1);
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
                    path_data: if empty {
                        String::new()
                    } else {
                        let inset = shape % 5;
                        format!(
                            "M{inset},0 L{},{} L0,{} Z",
                            8 + shape % 11,
                            8 + shape % 13,
                            8 + shape % 17,
                        )
                    },
                    unicode_values: Vec::new(),
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
            path_data: path_data.to_owned(),
            unicode_values: Vec::new(),
            width,
        }
    }

    fn font_tables(glyphs: Vec<ProcessedGlyph>) -> SerializedFontTables {
        generate_ttf_font_from_glyphs(
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
