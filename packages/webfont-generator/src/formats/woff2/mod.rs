use std::collections::{HashMap, HashSet};
use std::io::Error;

use crate::sfnt::SerializedFontTables;

mod compress;
mod glyf;
mod prepare;
mod serialize;
#[cfg(test)]
mod tests;

#[derive(Clone, Default)]
pub(crate) struct Woff2TransformCache {
    entries: HashMap<u64, Woff2TransformPayload>,
    #[cfg(test)]
    pub compile_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Woff2TransformPayload {
    pub transformed: Vec<u8>,
    pub normalized_glyf_len: usize,
    pub normalized_glyf_checksum: u32,
    pub normalized_loca_format: i16,
    pub normalized_loca_len: usize,
    pub normalized_loca_checksum: u32,
}

impl Woff2TransformCache {
    pub(super) fn transformed(&self, key: &u64) -> Option<Woff2TransformPayload> {
        self.entries.get(key).cloned()
    }
    pub(super) fn insert(&mut self, key: u64, payload: Woff2TransformPayload) {
        #[cfg(test)]
        {
            self.compile_count += 1;
        }
        self.entries.insert(key, payload);
    }
    pub(super) fn retain(&mut self, used_keys: &HashSet<u64>) {
        self.entries.retain(|key, _| used_keys.contains(key));
    }
}

pub(crate) fn tables_to_woff2(
    tables: &SerializedFontTables,
    quality: u8,
    cache: Option<&mut Woff2TransformCache>,
) -> Result<Vec<u8>, Error> {
    encode(tables, quality, cache)
}

fn encode(
    tables: &SerializedFontTables,
    quality: u8,
    cache: Option<&mut Woff2TransformCache>,
) -> Result<Vec<u8>, Error> {
    let mut prepared = prepare::prepare(tables, cache)?;
    let compressed = compress::compress_stream(std::mem::take(&mut prepared.stream), quality)?;
    serialize::assemble(&prepared, &compressed)
}

#[cfg(test)]
pub(crate) fn ttf_to_woff2(ttf: &[u8], quality: u8) -> Result<Vec<u8>, Error> {
    use std::io::ErrorKind;

    ::woff::version2::compress(ttf, "", quality.min(11) as usize, true)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 compression failed"))
}

#[cfg(feature = "bench")]
pub(crate) struct PreparedWoff2(prepare::PreparedWoff2);

#[cfg(feature = "bench")]
pub(crate) fn prepare_woff2(
    tables: &SerializedFontTables,
    cache: &mut Woff2TransformCache,
) -> Result<PreparedWoff2, Error> {
    prepare::prepare(tables, Some(cache)).map(PreparedWoff2)
}

#[cfg(feature = "bench")]
pub(crate) fn compress_prepared_woff2(
    prepared: &PreparedWoff2,
    quality: u8,
) -> Result<usize, Error> {
    compress::compress(&prepared.0, quality).map(|compressed| compressed.len())
}
