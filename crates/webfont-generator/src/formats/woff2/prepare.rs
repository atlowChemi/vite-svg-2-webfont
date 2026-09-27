use std::collections::HashSet;
use std::io::{Error, ErrorKind};

use write_fonts::read::tables::compute_checksum;
use write_fonts::read::tables::head::Head;
use write_fonts::read::tables::maxp::Maxp;
use write_fonts::read::{FontData, FontRead};

use crate::byte_helpers::BigEndian;
use crate::sfnt::{SerializedFontTables, SerializedTable};

use super::Woff2TransformCache;
use super::glyf::{
    GLYF_ENCODER_VERSION, NormalizedGlyfLoca, invalid_data, transform_cache_key,
    transform_glyf_loca,
};
use super::serialize::write_directory_entry;

pub(super) struct PreparedWoff2 {
    pub(super) directory: Vec<u8>,
    pub(super) stream: Vec<u8>,
    pub(super) table_count: u16,
    pub(super) total_sfnt_size: u32,
}

pub(super) fn prepare(
    tables: &SerializedFontTables,
    mut cache: Option<&mut Woff2TransformCache>,
) -> Result<PreparedWoff2, Error> {
    let mut ordered = Vec::with_capacity(tables.tables().len());
    let mut glyf = None;
    let mut loca = None;
    let mut head = None;
    let mut maxp = None;
    for table in tables.tables() {
        let required = match &table.tag {
            b"glyf" => Some(&mut glyf),
            b"loca" => Some(&mut loca),
            b"head" => Some(&mut head),
            b"maxp" => Some(&mut maxp),
            b"DSIG" => continue,
            _ => None,
        };
        if let Some(required) = required
            && required.replace(table).is_some()
        {
            return Err(invalid_data("duplicate required WOFF2 table"));
        }
        ordered.push(table);
    }
    if ordered.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput, "WOFF2 requires tables"));
    }
    let (Some(glyf), Some(loca), Some(head), Some(maxp)) = (glyf, loca, head, maxp) else {
        return Err(invalid_data(
            "transformed WOFF2 requires head, maxp, glyf, and loca",
        ));
    };
    ordered.sort_unstable_by_key(|table| table.tag);
    move_loca_after_glyf(&mut ordered);
    let head_table =
        Head::read(FontData::new(&head.bytes)).map_err(|_| invalid_data("invalid head table"))?;
    let maxp_table =
        Maxp::read(FontData::new(&maxp.bytes)).map_err(|_| invalid_data("invalid maxp table"))?;
    let index_format = head_table.index_to_loc_format();
    if !matches!(index_format, 0 | 1) {
        return Err(invalid_data("head indexToLocFormat must be 0 or 1"));
    }
    let num_glyphs = maxp_table.num_glyphs();
    let expected_loca_len = (usize::from(num_glyphs) + 1) * if index_format == 0 { 2 } else { 4 };
    if loca.bytes.len() != expected_loca_len {
        return Err(invalid_data("maxp numGlyphs does not match loca length"));
    }

    let key = transform_cache_key(
        *b"glyf",
        GLYF_ENCODER_VERSION,
        &[
            b"woff2-glyf-loca",
            &glyf.bytes,
            &loca.bytes,
            &index_format.to_be_bytes(),
            &num_glyphs.to_be_bytes(),
        ],
    );
    let payload = if let Some(hit) = cache.as_deref().and_then(|cache| cache.transformed(&key)) {
        hit
    } else {
        let payload = transform_glyf_loca(glyf, loca, index_format, num_glyphs)?;
        if let Some(cache) = cache.as_deref_mut() {
            cache.insert(key, payload.clone());
        }
        payload
    };
    if let Some(cache) = cache {
        cache.retain(&HashSet::from([key]));
    }

    let normalized = NormalizedGlyfLoca {
        glyf_len: payload.normalized_glyf_len,
        glyf_checksum: payload.normalized_glyf_checksum,
        loca_format: payload.normalized_loca_format,
        loca_len: payload.normalized_loca_len,
        loca_checksum: payload.normalized_loca_checksum,
    };
    let normalized_head = normalized_head(&ordered, &normalized)?;
    let table_count =
        u16::try_from(ordered.len()).map_err(|_| invalid_data("WOFF2 table count exceeds u16"))?;
    let stream_capacity = ordered.iter().try_fold(0_usize, |length, table| {
        let table_length = match &table.tag {
            b"glyf" => payload.transformed.len(),
            b"loca" => 0,
            b"head" => normalized_head.len(),
            _ => table.bytes.len(),
        };
        length
            .checked_add(table_length)
            .ok_or_else(|| invalid_data("WOFF2 stream size overflow"))
    })?;
    let mut directory = Vec::with_capacity(usize::from(table_count) * 11);
    let mut stream = Vec::with_capacity(stream_capacity);
    for table in &ordered {
        write_directory_entry(
            &mut directory,
            table,
            payload.transformed.len(),
            normalized.glyf_len,
            normalized.loca_len,
        )?;
        match &table.tag {
            b"glyf" => stream.extend_from_slice(&payload.transformed),
            b"loca" => {}
            b"head" => stream.extend_from_slice(&normalized_head),
            _ => stream.extend_from_slice(&table.bytes),
        }
    }

    Ok(PreparedWoff2 {
        directory,
        stream,
        table_count,
        total_sfnt_size: total_sfnt_size(&ordered, &normalized)?,
    })
}

fn total_sfnt_size(
    tables: &[&SerializedTable],
    normalized: &NormalizedGlyfLoca,
) -> Result<u32, Error> {
    12_usize
        .checked_add(16 * tables.len())
        .and_then(|size| {
            tables.iter().try_fold(size, |size, table| {
                let length = match &table.tag {
                    b"glyf" => normalized.glyf_len,
                    b"loca" => normalized.loca_len,
                    _ => table.bytes.len(),
                };
                size.checked_add((length + 3) & !3)
            })
        })
        .and_then(|size| u32::try_from(size).ok())
        .ok_or_else(|| invalid_data("SFNT size exceeds u32"))
}

fn normalized_head(
    tables: &[&SerializedTable],
    normalized: &NormalizedGlyfLoca,
) -> Result<Vec<u8>, Error> {
    let head = tables
        .iter()
        .find(|table| table.tag == *b"head")
        .ok_or_else(|| invalid_data("WOFF2 requires a head table"))?;
    if head.bytes.len() < 52 {
        return Err(invalid_data("head table is too short"));
    }

    let mut bytes = head.bytes.clone();
    bytes[8..12].fill(0);
    let flags = u16::from_be_bytes(bytes[16..18].try_into().unwrap()) | (1 << 11);
    let mut writer = BigEndian::new(&mut bytes);
    writer.write_u16_at(16, flags);
    writer.write_i16_at(50, normalized.loca_format);
    let head_checksum = compute_checksum(&bytes);

    let table_count =
        u32::try_from(tables.len()).map_err(|_| invalid_data("WOFF2 table count exceeds u32"))?;
    let max_power = if table_count == 0 {
        0
    } else {
        1 << table_count.ilog2()
    };
    let search_range = max_power * 16;
    let entry_selector = if table_count == 0 {
        0
    } else {
        table_count.ilog2()
    };
    let range_shift = table_count * 16 - search_range;
    let mut checksum = 0x0001_0000_u32
        .wrapping_add((table_count << 16).wrapping_add(search_range))
        .wrapping_add((entry_selector << 16).wrapping_add(range_shift));
    let mut offset = 12_usize
        .checked_add(
            16_usize
                .checked_mul(tables.len())
                .ok_or_else(|| invalid_data("SFNT size overflow"))?,
        )
        .ok_or_else(|| invalid_data("SFNT size overflow"))?;
    for table in tables {
        let (table_checksum, table_len) = match &table.tag {
            b"head" => (head_checksum, table.bytes.len()),
            b"glyf" => (normalized.glyf_checksum, normalized.glyf_len),
            b"loca" => (normalized.loca_checksum, normalized.loca_len),
            _ => (table.checksum, table.bytes.len()),
        };
        checksum = checksum
            .wrapping_add(u32::from_be_bytes(table.tag))
            .wrapping_add(table_checksum)
            .wrapping_add(
                u32::try_from(offset).map_err(|_| invalid_data("SFNT offset exceeds u32"))?,
            )
            .wrapping_add(
                u32::try_from(table_len).map_err(|_| invalid_data("table size exceeds u32"))?,
            )
            .wrapping_add(table_checksum);
        offset = offset
            .checked_add((table_len + 3) & !3)
            .ok_or_else(|| invalid_data("SFNT size overflow"))?;
    }
    BigEndian::new(&mut bytes).write_u32_at(8, 0xb1b0_afba_u32.wrapping_sub(checksum));
    Ok(bytes)
}

fn move_loca_after_glyf(tables: &mut Vec<&SerializedTable>) {
    let loca_index = tables
        .iter()
        .position(|table| table.tag == *b"loca")
        .expect("required loca table must be present");
    let loca = tables.remove(loca_index);
    let glyf_index = tables
        .iter()
        .position(|table| table.tag == *b"glyf")
        .expect("required glyf table must be present");
    tables.insert(glyf_index + 1, loca);
}
