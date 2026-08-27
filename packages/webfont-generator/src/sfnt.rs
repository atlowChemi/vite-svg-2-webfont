use std::io::{Error, ErrorKind};
use std::sync::{Arc, OnceLock};

use crate::byte_helpers::BigEndian;
use write_fonts::read::tables::compute_checksum;

const CHECKSUM_ADJUSTMENT: u32 = 0xb1b0_afba;
const HEAD_CHECKSUM_ADJUSTMENT_OFFSET: usize = 8;
const HEAD_TAG: [u8; 4] = *b"head";
const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_ENTRY_SIZE: usize = 16;
const TT_SFNT_VERSION: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

#[derive(Clone)]
pub(crate) struct SerializedFontTables {
    tables: Vec<SerializedTable>,
    ttf: OnceLock<Arc<Vec<u8>>>,
}

#[derive(Clone)]
pub(crate) struct SerializedTable {
    pub tag: [u8; 4],
    pub checksum: u32,
    pub bytes: Vec<u8>,
}

impl SerializedFontTables {
    pub fn new(tables: Vec<([u8; 4], Vec<u8>)>) -> Result<Self, Error> {
        if tables.len() > u16::MAX as usize {
            return Err(Error::new(ErrorKind::InvalidInput, "Too many SFNT tables."));
        }
        let mut tables = tables;
        tables.sort_unstable_by_key(|(tag, _)| table_order_key(tag));
        let mut tables = tables
            .into_iter()
            .map(|(tag, mut bytes)| {
                if tag == HEAD_TAG && bytes.len() >= HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4 {
                    bytes[HEAD_CHECKSUM_ADJUSTMENT_OFFSET..HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4]
                        .copy_from_slice(&[0, 0, 0, 0]);
                }
                SerializedTable {
                    tag,
                    checksum: compute_checksum(&bytes),
                    bytes,
                }
            })
            .collect::<Vec<_>>();
        apply_checksum_adjustment(&mut tables);
        let ttf = OnceLock::new();
        Ok(Self { tables, ttf })
    }

    pub fn ttf(&self) -> &[u8] {
        self.ttf
            .get_or_init(|| Arc::new(build_sfnt(&self.tables)))
            .as_slice()
    }

    pub fn ttf_arc(&self) -> Arc<Vec<u8>> {
        Arc::clone(self.ttf.get_or_init(|| Arc::new(build_sfnt(&self.tables))))
    }

    pub fn tables(&self) -> &[SerializedTable] {
        &self.tables
    }

    #[cfg(feature = "bench")]
    pub(crate) fn uncached_ttf(&self) -> Vec<u8> {
        build_sfnt(&self.tables)
    }

    #[cfg(feature = "bench")]
    pub(crate) fn clone_raw_tables(&self) -> Vec<([u8; 4], Vec<u8>)> {
        self.tables
            .iter()
            .map(|table| (table.tag, table.bytes.clone()))
            .collect()
    }
}

fn table_order_key(tag: &[u8; 4]) -> (u8, usize, [u8; 4]) {
    // Recommended TTF table order (OpenType spec). DSIG sorts last, unknown
    // tables sort after the recommended set but before DSIG.
    let recommended = match tag {
        b"head" => 0,
        b"hhea" => 1,
        b"maxp" => 2,
        b"OS/2" => 3,
        b"hmtx" => 4,
        b"LTSH" => 5,
        b"VDMX" => 6,
        b"hdmx" => 7,
        b"cmap" => 8,
        b"fpgm" => 9,
        b"prep" => 10,
        b"cvt " => 11,
        b"loca" => 12,
        b"glyf" => 13,
        b"kern" => 14,
        b"name" => 15,
        b"post" => 16,
        b"gasp" => 17,
        b"PCLT" => 18,
        b"DSIG" => return (2, 0, *tag),
        _ => return (1, 0, *tag),
    };
    (0, recommended, *tag)
}

fn apply_checksum_adjustment(tables: &mut [SerializedTable]) {
    let checksum_adjustment = checksum_adjustment(tables);
    if let Some(head) = tables.iter_mut().find(|table| table.tag == HEAD_TAG)
        && head.bytes.len() >= HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4
    {
        BigEndian::new(&mut head.bytes)
            .write_u32_at(HEAD_CHECKSUM_ADJUSTMENT_OFFSET, checksum_adjustment);
    }
}

fn checksum_adjustment(tables: &[SerializedTable]) -> u32 {
    let directory_checksum = compute_checksum(&sfnt_directory(tables));
    let table_checksum = tables
        .iter()
        .map(|table| table.checksum)
        .fold(0_u32, u32::wrapping_add);
    CHECKSUM_ADJUSTMENT.wrapping_sub(table_checksum.wrapping_add(directory_checksum))
}

fn build_sfnt(tables: &[SerializedTable]) -> Vec<u8> {
    let mut bytes = sfnt_directory(tables);
    bytes.reserve(
        tables
            .iter()
            .map(|table| align4_len(table.bytes.len()))
            .sum(),
    );
    for table in tables {
        bytes.extend_from_slice(&table.bytes);
        align4(&mut bytes);
    }
    bytes
}

fn sfnt_directory(tables: &[SerializedTable]) -> Vec<u8> {
    let mut offset = SFNT_HEADER_SIZE + tables.len() * SFNT_TABLE_ENTRY_SIZE;
    let mut records = Vec::with_capacity(tables.len());
    for table in tables {
        records.push((table.tag, table.checksum, offset, table.bytes.len()));
        offset += align4_len(table.bytes.len());
    }
    records.sort_unstable_by_key(|record| record.0);

    let mut directory =
        Vec::with_capacity(SFNT_HEADER_SIZE + records.len() * SFNT_TABLE_ENTRY_SIZE);
    directory.extend_from_slice(&TT_SFNT_VERSION);
    {
        let mut dir_writer = BigEndian::new(&mut directory);
        dir_writer.push_u16(tables.len() as u16);
        let (search_range, entry_selector, range_shift) =
            search_range(tables.len(), SFNT_TABLE_ENTRY_SIZE);
        dir_writer.push_u16(search_range);
        dir_writer.push_u16(entry_selector);
        dir_writer.push_u16(range_shift);
    }
    for (tag, checksum, offset, length) in records {
        directory.extend_from_slice(&tag);
        let mut dir_writer = BigEndian::new(&mut directory);
        dir_writer.push_u32(checksum);
        dir_writer.push_u32(offset as u32);
        dir_writer.push_u32(length as u32);
    }
    directory
}

fn search_range(item_count: usize, item_size: usize) -> (u16, u16, u16) {
    if item_count == 0 || item_size == 0 {
        return (0, 0, 0);
    }

    let entry_selector = usize::BITS as usize - 1 - item_count.leading_zeros() as usize;
    let search_range = (1_usize << entry_selector) * item_size;
    let range_shift = item_count * item_size - search_range;
    (
        search_range as u16,
        entry_selector as u16,
        range_shift as u16,
    )
}

fn align4(bytes: &mut Vec<u8>) {
    bytes.resize(align4_len(bytes.len()), 0);
}

fn align4_len(len: usize) -> usize {
    (len + 3) & !3
}
