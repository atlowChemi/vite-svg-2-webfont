use std::io::{Error, ErrorKind, Write};

use crate::sfnt::SerializedFontTables;
use crate::ttf::TtfGlyphCache;
use flate2::Compression;
use flate2::write::ZlibEncoder;

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
    cache: &mut TtfGlyphCache,
) -> Result<Vec<u8>, Error> {
    let mut woff_buf = encode_woff1(tables, Some(cache))?;
    if let Some(metadata) = metadata {
        inject_woff_metadata(&mut woff_buf, metadata)?;
    }
    Ok(woff_buf)
}

/// Encodes `ttf` as WOFF2. `quality` is the Brotli compression quality (0–11); callers
/// are expected to have validated the range (see `validate_generate_webfonts_options`).
pub(crate) fn ttf_to_woff2(ttf: &[u8], quality: u8) -> Result<Vec<u8>, Error> {
    ::woff::version2::compress(ttf, "", quality.min(11) as usize, true)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 compression failed"))
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
    mut cache: Option<&mut TtfGlyphCache>,
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

fn woff1_payload_cache_key(tag: [u8; 4], bytes: &[u8]) -> [u8; 16] {
    let mut key = Vec::with_capacity(4 + bytes.len());
    key.extend_from_slice(&tag);
    key.extend_from_slice(bytes);
    md5::compute(key).0
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
    use crate::test_helpers::fixture_font_tables;
    use flate2::read::ZlibDecoder;
    use std::io::Read;

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
}
