use std::io::{Error, ErrorKind};

use crate::byte_helpers::BigEndian;
use crate::sfnt::SerializedTable;

use super::glyf::invalid_data;
use super::prepare::PreparedWoff2;

pub(super) const HEADER_SIZE: usize = 48;
pub(super) const KNOWN_TAGS: [[u8; 4]; 63] = [
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"OS/2", *b"post", *b"cvt ",
    *b"fpgm", *b"glyf", *b"loca", *b"prep", *b"CFF ", *b"VORG", *b"EBDT", *b"EBLC", *b"gasp",
    *b"hdmx", *b"kern", *b"LTSH", *b"PCLT", *b"VDMX", *b"vhea", *b"vmtx", *b"BASE", *b"GDEF",
    *b"GPOS", *b"GSUB", *b"EBSC", *b"JSTF", *b"MATH", *b"CBDT", *b"CBLC", *b"COLR", *b"CPAL",
    *b"SVG ", *b"sbix", *b"acnt", *b"avar", *b"bdat", *b"bloc", *b"bsln", *b"cvar", *b"fdsc",
    *b"feat", *b"fmtx", *b"fvar", *b"gvar", *b"hsty", *b"just", *b"lcar", *b"mort", *b"morx",
    *b"opbd", *b"prop", *b"trak", *b"Zapf", *b"Silf", *b"Glat", *b"Gloc", *b"Feat", *b"Sill",
];

pub(super) fn assemble(prepared: &PreparedWoff2, compressed: &[u8]) -> Result<Vec<u8>, Error> {
    let compressed_size = u32::try_from(compressed.len())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "compressed stream exceeds u32"))?;
    let unaligned_length = HEADER_SIZE
        .checked_add(prepared.directory.len())
        .and_then(|size| size.checked_add(compressed.len()))
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 size overflow"))?;
    let length = unaligned_length
        .checked_add(3)
        .map(|size| size & !3)
        .and_then(|size| u32::try_from(size).ok())
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 size exceeds u32"))?;

    let mut output = Vec::with_capacity(length as usize);
    output.extend_from_slice(b"wOF2");
    {
        let mut writer = BigEndian::new(&mut output);
        writer.push_u32(0x0001_0000);
        writer.push_u32(length);
        writer.push_u16(prepared.table_count);
        writer.push_u16(0);
        writer.push_u32(prepared.total_sfnt_size);
        writer.push_u32(compressed_size);
        writer.push_u16(1);
        writer.push_u16(0);
    }
    output.extend_from_slice(&[0; 20]);
    output.extend_from_slice(&prepared.directory);
    output.extend_from_slice(compressed);
    output.resize(length as usize, 0);
    Ok(output)
}

pub(super) fn write_directory_entry(
    output: &mut Vec<u8>,
    table: &SerializedTable,
    transformed_glyf_len: usize,
    normalized_glyf_len: usize,
    normalized_loca_len: usize,
) -> Result<(), Error> {
    let index = KNOWN_TAGS.iter().position(|tag| tag == &table.tag);
    output.push(index.unwrap_or(63) as u8);
    if index.is_none() {
        output.extend_from_slice(&table.tag);
    }
    write_base128(
        u32::try_from(match &table.tag {
            b"glyf" => normalized_glyf_len,
            b"loca" => normalized_loca_len,
            _ => table.bytes.len(),
        })
        .map_err(|_| invalid_data("table size exceeds u32"))?,
        output,
    );
    if table.tag == *b"glyf" {
        write_base128(
            u32::try_from(transformed_glyf_len)
                .map_err(|_| invalid_data("transformed glyf size exceeds u32"))?,
            output,
        );
    } else if table.tag == *b"loca" {
        write_base128(0, output);
    }
    Ok(())
}

pub(super) fn write_base128(value: u32, output: &mut Vec<u8>) {
    let bits = (32 - value.leading_zeros()).max(1);
    let groups = bits.div_ceil(7);
    for group in (0..groups).rev() {
        let byte = ((value >> (group * 7)) & 0x7f) as u8;
        output.push(byte | u8::from(group != 0) << 7);
    }
}
