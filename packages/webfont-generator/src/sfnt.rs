use std::io::{Error, ErrorKind};
use std::sync::{Arc, OnceLock};

const CHECKSUM_ADJUSTMENT: u32 = 0xb1b0_afba;
const HEAD_CHECKSUM_ADJUSTMENT_OFFSET: usize = 8;
const HEAD_TAG: [u8; 4] = *b"head";
const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_ENTRY_SIZE: usize = 16;
const TT_SFNT_VERSION: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

const RECOMMENDED_TABLE_ORDER_TTF: [[u8; 4]; 19] = [
    *b"head", *b"hhea", *b"maxp", *b"OS/2", *b"hmtx", *b"LTSH", *b"VDMX", *b"hdmx", *b"cmap",
    *b"fpgm", *b"prep", *b"cvt ", *b"loca", *b"glyf", *b"kern", *b"name", *b"post", *b"gasp",
    *b"PCLT",
];

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
                    checksum: checksum(&bytes),
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
}

fn table_order_key(tag: &[u8; 4]) -> (u8, usize, [u8; 4]) {
    if tag == b"DSIG" {
        return (2, 0, *tag);
    }
    if let Some(index) = RECOMMENDED_TABLE_ORDER_TTF
        .iter()
        .position(|ordered| ordered == tag)
    {
        return (0, index, *tag);
    }
    (1, 0, *tag)
}

fn apply_checksum_adjustment(tables: &mut [SerializedTable]) {
    let checksum_adjustment = checksum_adjustment(tables);
    if let Some(head) = tables.iter_mut().find(|table| table.tag == HEAD_TAG)
        && head.bytes.len() >= HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4
    {
        head.bytes[HEAD_CHECKSUM_ADJUSTMENT_OFFSET..HEAD_CHECKSUM_ADJUSTMENT_OFFSET + 4]
            .copy_from_slice(&checksum_adjustment.to_be_bytes());
    }
}

fn checksum_adjustment(tables: &[SerializedTable]) -> u32 {
    let directory_checksum = checksum(&sfnt_directory(tables));
    let table_checksum = tables
        .iter()
        .map(|table| table.checksum)
        .fold(0_u32, u32::wrapping_add);
    CHECKSUM_ADJUSTMENT.wrapping_sub(table_checksum.wrapping_add(directory_checksum))
}

fn build_sfnt(tables: &[SerializedTable]) -> Vec<u8> {
    let mut bytes = sfnt_directory(tables);
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
        records.push((table.tag, offset, table.bytes.len()));
        offset += align4_len(table.bytes.len());
    }
    records.sort_unstable_by_key(|record| record.0);

    let mut directory =
        Vec::with_capacity(SFNT_HEADER_SIZE + records.len() * SFNT_TABLE_ENTRY_SIZE);
    directory.extend_from_slice(&TT_SFNT_VERSION);
    write_u16_be(&mut directory, tables.len() as u16);
    let (search_range, entry_selector, range_shift) =
        search_range(tables.len(), SFNT_TABLE_ENTRY_SIZE);
    write_u16_be(&mut directory, search_range);
    write_u16_be(&mut directory, entry_selector);
    write_u16_be(&mut directory, range_shift);
    for (tag, offset, length) in records {
        let table = tables.iter().find(|table| table.tag == tag).unwrap();
        directory.extend_from_slice(&tag);
        write_u32_be(&mut directory, table.checksum);
        write_u32_be(&mut directory, offset as u32);
        write_u32_be(&mut directory, length as u32);
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

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0_u32, |sum, chunk| {
        let mut bytes = [0_u8; 4];
        bytes[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(bytes))
    })
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
