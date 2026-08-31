use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::io::{Error, ErrorKind, Write};

use crate::byte_helpers::BigEndian;
use crate::sfnt::SerializedFontTables;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use rustc_hash::FxHasher;

const WOFF_HEADER_SIZE: usize = 44;
const WOFF_TABLE_ENTRY_SIZE: usize = 20;
const META_OFFSET_POS: usize = 24;
const META_LENGTH_POS: usize = 28;
const META_ORIG_LENGTH_POS: usize = 32;
const LENGTH_POS: usize = 8;
const WOFF_SIGNATURE: [u8; 4] = *b"wOFF";

#[derive(Clone, Default)]
pub(crate) struct Woff1PayloadCache {
    entries: HashMap<u64, Vec<u8>>,
    #[cfg(test)]
    compile_count: usize,
}

impl Woff1PayloadCache {
    pub(crate) fn woff1_payload(&self, key: &u64) -> Option<Vec<u8>> {
        self.entries.get(key).cloned()
    }
    pub(crate) fn insert_woff1_payload(&mut self, key: u64, payload: Vec<u8>) {
        #[cfg(test)]
        {
            self.compile_count += 1;
        }
        self.entries.insert(key, payload);
    }
    pub(crate) fn retain_woff1_payloads(&mut self, used_keys: &HashSet<u64>) {
        self.entries.retain(|key, _| used_keys.contains(key));
    }
    #[cfg(test)]
    pub(crate) fn compile_count(&self) -> usize {
        self.compile_count
    }
    #[cfg(feature = "bench")]
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

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

    let mut writer = BigEndian::new(woff);
    writer.write_u32_at(LENGTH_POS, total_length);
    writer.write_u32_at(META_OFFSET_POS, meta_offset);
    writer.write_u32_at(META_LENGTH_POS, meta_length);
    writer.write_u32_at(META_ORIG_LENGTH_POS, meta_orig_length);

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
    {
        let mut woff_writer = BigEndian::new(&mut woff);
        woff_writer.push_u32(total_length as u32);
        woff_writer.push_u16(table_count as u16);
        woff_writer.push_u16(0);
        woff_writer.push_u32(total_sfnt_size(tables));
        woff_writer.push_u16(1);
        woff_writer.push_u16(0);
        woff_writer.push_u32(0);
        woff_writer.push_u32(0);
        woff_writer.push_u32(0);
        woff_writer.push_u32(0);
        woff_writer.push_u32(0);
    }

    for (tag, offset, comp_length, orig_length, checksum) in entries {
        woff.extend_from_slice(&tag);
        let mut woff_writer = BigEndian::new(&mut woff);
        woff_writer.push_u32(offset);
        woff_writer.push_u32(comp_length);
        woff_writer.push_u32(orig_length);
        woff_writer.push_u32(checksum);
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

#[cfg(test)]
mod tests;
