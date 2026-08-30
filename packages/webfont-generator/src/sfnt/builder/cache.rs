use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::io::Error;

use rustc_hash::FxHasher;
use write_fonts::read::TopLevelTable;
use write_fonts::validate::Validate;
use write_fonts::{FontWrite, dump_table};

use crate::pipeline::TtfGlyphCache;
use crate::svg::types::ProcessedGlyph;

#[derive(Clone, Default)]
pub(crate) struct Woff1PayloadCache {
    entries: HashMap<u64, Vec<u8>>,
    #[cfg(test)]
    compile_count: usize,
}

#[derive(Clone, Default)]
pub(crate) struct Woff2TransformCache {
    entries: HashMap<u64, Woff2TransformPayload>,
    #[cfg(test)]
    pub compile_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Woff2TransformPayload {
    pub transformed: Vec<u8>,
    pub normalized_glyf_len: usize,
    pub normalized_glyf_checksum: u32,
    pub normalized_loca_format: i16,
    pub normalized_loca_len: usize,
    pub normalized_loca_checksum: u32,
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

impl Woff2TransformCache {
    pub(crate) fn transformed(&self, key: &u64) -> Option<Woff2TransformPayload> {
        self.entries.get(key).cloned()
    }
    pub(crate) fn insert(&mut self, key: u64, payload: Woff2TransformPayload) {
        #[cfg(test)]
        {
            self.compile_count += 1;
        }
        self.entries.insert(key, payload);
    }
    pub(crate) fn retain(&mut self, used_keys: &HashSet<u64>) {
        self.entries.retain(|key, _| used_keys.contains(key));
    }
}

pub(super) fn compiled_glyph_cache_key(glyph: &ProcessedGlyph, advance_width: u16) -> u64 {
    let mut hasher = FxHasher::default();
    match glyph.ttf_path_hash {
        Some(path_hash) => hasher.write_u64(path_hash),
        None => hasher.write(glyph.path_data.as_bytes()),
    }
    hasher.write_u16(advance_width);
    hasher.finish()
}

pub(super) fn dump_cached_ttf_table<T>(
    cache: &mut Option<&mut TtfGlyphCache>,
    used_table_keys: &mut HashSet<u64>,
    cache_key: impl FnOnce() -> u64,
    table: &T,
    name: &str,
) -> Result<([u8; 4], Vec<u8>), Error>
where
    T: FontWrite + TopLevelTable + Validate,
{
    let Some(cache) = cache.as_deref_mut() else {
        return dump_ttf_table(table, name);
    };
    let cache_key = cache_key();
    used_table_keys.insert(cache_key);
    if let Some(cached) = cache.tables.get(&cache_key) {
        return Ok(cached.clone());
    }
    #[cfg(test)]
    {
        cache.table_compile_count += 1;
    }
    let dumped = dump_ttf_table(table, name)?;
    cache.tables.insert(cache_key, dumped.clone());
    Ok(dumped)
}

pub(super) fn table_cache_key(tag: &[u8; 4], hash_inputs: impl FnOnce(&mut FxHasher)) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(tag);
    hash_inputs(&mut hasher);
    hasher.finish()
}
pub(super) fn hash_option_str(hasher: &mut FxHasher, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.write_u8(1);
            hash_str(hasher, value);
        }
        None => hasher.write_u8(0),
    }
}
pub(super) fn hash_str(hasher: &mut FxHasher, value: &str) {
    hasher.write_usize(value.len());
    hasher.write(value.as_bytes());
}
pub(super) fn dump_ttf_table<T>(table: &T, name: &str) -> Result<([u8; 4], Vec<u8>), Error>
where
    T: FontWrite + TopLevelTable + Validate,
{
    dump_table(table)
        .map(|bytes| (T::TAG.to_be_bytes(), bytes))
        .map_err(|error| Error::other(format!("Failed to add {name} table: {error:?}")))
}
