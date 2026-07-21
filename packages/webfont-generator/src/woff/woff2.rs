use std::collections::HashSet;
use std::ffi::c_int;
use std::hash::Hasher;
use std::io::{Error, ErrorKind};

use crate::sfnt::{SerializedFontTables, SerializedTable};
use crate::ttf::Woff2TransformCache;
use rustc_hash::FxHasher;

const HEADER_SIZE: usize = 48;
const KNOWN_TAGS: [[u8; 4]; 63] = [
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"OS/2", *b"post", *b"cvt ",
    *b"fpgm", *b"glyf", *b"loca", *b"prep", *b"CFF ", *b"VORG", *b"EBDT", *b"EBLC", *b"gasp",
    *b"hdmx", *b"kern", *b"LTSH", *b"PCLT", *b"VDMX", *b"vhea", *b"vmtx", *b"BASE", *b"GDEF",
    *b"GPOS", *b"GSUB", *b"EBSC", *b"JSTF", *b"MATH", *b"CBDT", *b"CBLC", *b"COLR", *b"CPAL",
    *b"SVG ", *b"sbix", *b"acnt", *b"avar", *b"bdat", *b"bloc", *b"bsln", *b"cvar", *b"fdsc",
    *b"feat", *b"fmtx", *b"fvar", *b"gvar", *b"hsty", *b"just", *b"lcar", *b"mort", *b"morx",
    *b"opbd", *b"prop", *b"trak", *b"Zapf", *b"Silf", *b"Glat", *b"Gloc", *b"Feat", *b"Sill",
];

unsafe extern "C" {
    fn BrotliEncoderCompress(
        quality: c_int,
        lgwin: c_int,
        mode: c_int,
        input_size: usize,
        input: *const u8,
        encoded_size: *mut usize,
        encoded: *mut u8,
    ) -> c_int;
}

pub(super) struct PreparedWoff2 {
    directory: Vec<u8>,
    stream: Vec<u8>,
    table_count: u16,
    total_sfnt_size: u32,
}

pub(super) fn encode(tables: &SerializedFontTables, quality: u8) -> Result<Vec<u8>, Error> {
    let prepared = prepare(tables, None)?;
    let compressed = compress(&prepared, quality)?;
    assemble(&prepared, &compressed)
}

pub(super) fn prepare(
    tables: &SerializedFontTables,
    cache: Option<&mut Woff2TransformCache>,
) -> Result<PreparedWoff2, Error> {
    let mut ordered = tables
        .tables()
        .iter()
        .filter(|table| table.tag != *b"DSIG")
        .collect::<Vec<_>>();
    if ordered.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput, "WOFF2 requires tables"));
    }
    ordered.sort_unstable_by_key(|table| table.tag);
    move_loca_after_glyf(&mut ordered);

    let mut directory = Vec::new();
    let mut stream = Vec::new();
    for table in &ordered {
        write_directory_entry(&mut directory, table)?;
        if table.tag == *b"head" {
            let mut head = table.bytes.clone();
            if head.len() < 18 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "head table is too short",
                ));
            }
            let flags = u16::from_be_bytes(head[16..18].try_into().unwrap()) | (1 << 11);
            head[16..18].copy_from_slice(&flags.to_be_bytes());
            stream.extend_from_slice(&head);
        } else {
            stream.extend_from_slice(&table.bytes);
        }
    }

    let total_sfnt_size = 12_usize
        .checked_add(16 * ordered.len())
        .and_then(|size| {
            ordered.iter().try_fold(size, |size, table| {
                size.checked_add((table.bytes.len() + 3) & !3)
            })
        })
        .and_then(|size| u32::try_from(size).ok())
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "SFNT size exceeds u32"))?;
    if let Some(cache) = cache {
        cache.retain(&HashSet::new());
    }
    Ok(PreparedWoff2 {
        directory,
        stream,
        table_count: ordered.len() as u16,
        total_sfnt_size,
    })
}

pub(super) fn compress(prepared: &PreparedWoff2, quality: u8) -> Result<Vec<u8>, Error> {
    google_brotli_compress(&prepared.stream, quality.min(11))
}

fn assemble(prepared: &PreparedWoff2, compressed: &[u8]) -> Result<Vec<u8>, Error> {
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
    output.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(&prepared.table_count.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&prepared.total_sfnt_size.to_be_bytes());
    output.extend_from_slice(&compressed_size.to_be_bytes());
    output.extend_from_slice(&1_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&[0; 20]);
    output.extend_from_slice(&prepared.directory);
    output.extend_from_slice(compressed);
    output.resize(length as usize, 0);
    Ok(output)
}

#[cfg_attr(not(test), allow(dead_code))]
fn transform_cache_key(tag: [u8; 4], transform_version: u8, inputs: &[&[u8]]) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(&tag);
    hasher.write_u8(transform_version);
    for input in inputs {
        hasher.write(input);
    }
    hasher.finish()
}

fn move_loca_after_glyf(tables: &mut Vec<&SerializedTable>) {
    let Some(loca_index) = tables.iter().position(|table| table.tag == *b"loca") else {
        return;
    };
    let loca = tables.remove(loca_index);
    if let Some(glyf_index) = tables.iter().position(|table| table.tag == *b"glyf") {
        tables.insert(glyf_index + 1, loca);
    } else {
        tables.insert(loca_index.min(tables.len()), loca);
    }
}

fn write_directory_entry(output: &mut Vec<u8>, table: &SerializedTable) -> Result<(), Error> {
    let index = KNOWN_TAGS.iter().position(|tag| tag == &table.tag);
    let transform = u8::from(matches!(&table.tag, b"glyf" | b"loca")) * 3;
    output.push(index.unwrap_or(63) as u8 | transform << 6);
    if index.is_none() {
        output.extend_from_slice(&table.tag);
    }
    write_base128(
        u32::try_from(table.bytes.len())
            .map_err(|_| Error::new(ErrorKind::InvalidData, "table size exceeds u32"))?,
        output,
    );
    Ok(())
}

fn write_base128(value: u32, output: &mut Vec<u8>) {
    let bits = (32 - value.leading_zeros()).max(1);
    let groups = bits.div_ceil(7);
    for group in (0..groups).rev() {
        let byte = ((value >> (group * 7)) & 0x7f) as u8;
        output.push(byte | u8::from(group != 0) << 7);
    }
}

fn google_brotli_compress(input: &[u8], quality: u8) -> Result<Vec<u8>, Error> {
    let capacity = input
        .len()
        .checked_add(input.len() / 5)
        .and_then(|size| size.checked_add(10_240))
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Brotli input is too large"))?;
    let mut output = vec![0; capacity];
    let mut encoded_size = output.len();
    // `woff` links this temporary adapter's Google Brotli implementation.
    let success = unsafe {
        BrotliEncoderCompress(
            quality.into(),
            22,
            2,
            input.len(),
            input.as_ptr(),
            &mut encoded_size,
            output.as_mut_ptr(),
        )
    };
    if success == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Brotli compression failed",
        ));
    }
    output.truncate(encoded_size);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::fixture_font_tables;
    use write_fonts::read::tables::compute_checksum;
    use write_fonts::read::{FontRef, TableProvider};

    #[test]
    fn writes_canonical_base128_values() {
        for (value, expected) in [
            (0, &[0][..]),
            (127, &[0x7f]),
            (128, &[0x81, 0]),
            (16_383, &[0xff, 0x7f]),
            (16_384, &[0x81, 0x80, 0]),
            (u32::MAX, &[0x8f, 0xff, 0xff, 0xff, 0x7f]),
        ] {
            let mut actual = Vec::new();
            write_base128(value, &mut actual);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn writes_known_and_unknown_directory_entries() {
        for (tag, expected_flag) in [(*b"glyf", 0xca), (*b"loca", 0xcb)] {
            let mut output = Vec::new();
            write_directory_entry(
                &mut output,
                &SerializedTable {
                    tag,
                    checksum: 0,
                    bytes: vec![0; 127],
                },
            )
            .unwrap();
            assert_eq!(output, [expected_flag, 127]);
        }

        let mut output = Vec::new();
        write_directory_entry(
            &mut output,
            &SerializedTable {
                tag: *b"TEST",
                checksum: 0,
                bytes: vec![0; 128],
            },
        )
        .unwrap();
        assert_eq!(output, [63, b'T', b'E', b'S', b'T', 0x81, 0]);
    }

    #[test]
    fn transform_cache_key_hashes_tag_version_and_full_bodies() {
        let body = b"body".as_slice();
        let key = transform_cache_key(*b"glyf", 0, &[body]);
        assert_ne!(key, transform_cache_key(*b"loca", 0, &[body]));
        assert_ne!(key, transform_cache_key(*b"glyf", 1, &[body]));
        assert_ne!(key, transform_cache_key(*b"glyf", 0, &[b"Body"]));

        let first = [0, 0, 0, 1, 0, 0, 0, 2];
        let second = [0, 0, 0, 2, 0, 0, 0, 1];
        assert_eq!(compute_checksum(&first), compute_checksum(&second));
        assert_ne!(
            transform_cache_key(*b"glyf", 0, &[&first]),
            transform_cache_key(*b"glyf", 0, &[&second])
        );
    }

    #[test]
    fn transform_cache_hits_and_prunes_unused_entries() {
        let mut cache = Woff2TransformCache::default();
        cache.insert(1, vec![1]);
        cache.insert(2, vec![2]);
        assert_eq!(cache.transformed(&1), Some(vec![1]));
        assert_eq!(cache.compile_count, 2);

        cache.retain(&HashSet::from([2]));
        assert_eq!(cache.transformed(&1), None);
        assert_eq!(cache.transformed(&2), Some(vec![2]));
    }

    #[test]
    fn identity_preparation_does_not_populate_the_transform_cache() {
        let mut cache = Woff2TransformCache::default();
        prepare(&fixture_font_tables(), Some(&mut cache)).unwrap();
        assert_eq!(cache.compile_count, 0);
        assert_eq!(cache.transformed(&0), None);
    }

    #[test]
    fn split_preparation_compression_and_assembly_matches_encode() {
        let tables = fixture_font_tables();
        let prepared = prepare(&tables, None).unwrap();
        let compressed = compress(&prepared, 11).unwrap();
        assert_eq!(
            assemble(&prepared, &compressed).unwrap(),
            encode(&tables, 11).unwrap()
        );
    }

    #[test]
    fn no_transform_woff2_round_trips() {
        let tables = fixture_font_tables();
        let output = encode(&tables, 11).unwrap();
        assert_eq!(&output[..4], b"wOF2");
        assert_eq!(
            u32::from_be_bytes(output[8..12].try_into().unwrap()),
            output.len() as u32
        );
        assert_eq!(
            u16::from_be_bytes(output[12..14].try_into().unwrap()) as usize,
            tables.tables().len()
        );
        assert_directory(&output, &tables);

        let decoded = ::woff::version2::decompress(&output).expect("WOFF2 should decode");
        assert_same_semantics(tables.ttf(), &decoded);
    }

    #[test]
    fn removes_dsig_and_marks_the_font_as_transformed() {
        let tables = fixture_font_tables();
        let mut raw_tables = tables
            .tables()
            .iter()
            .map(|table| (table.tag, table.bytes.clone()))
            .collect::<Vec<_>>();
        raw_tables.push((*b"DSIG", vec![0; 8]));
        let tables = SerializedFontTables::new(raw_tables).unwrap();

        let output = encode(&tables, 11).unwrap();
        assert_eq!(
            u16::from_be_bytes(output[12..14].try_into().unwrap()) as usize,
            tables.tables().len() - 1
        );
        let decoded = ::woff::version2::decompress(&output).unwrap();
        assert!(sfnt_table(&decoded, b"DSIG").is_none());
        let head = sfnt_table(&decoded, b"head").unwrap();
        assert_ne!(
            u16::from_be_bytes(head[16..18].try_into().unwrap()) & (1 << 11),
            0
        );
    }

    #[test]
    fn no_transform_woff2_is_deterministic_and_matches_native_semantics() {
        let tables = fixture_font_tables();
        let internal = encode(&tables, 11).unwrap();
        assert_eq!(internal, encode(&tables, 11).unwrap());
        let native = ::woff::version2::compress(tables.ttf(), "", 11, false).unwrap();
        let internal = ::woff::version2::decompress(&internal).unwrap();
        let native = ::woff::version2::decompress(&native).unwrap();
        assert_same_semantics(tables.ttf(), &internal);
        assert_same_semantics(tables.ttf(), &native);
    }

    fn assert_same_semantics(source: &[u8], decoded: &[u8]) {
        let source = FontRef::new(source).expect("source should parse");
        let decoded = FontRef::new(decoded).expect("decoded font should parse");
        assert_eq!(
            source.maxp().unwrap().num_glyphs(),
            decoded.maxp().unwrap().num_glyphs()
        );
        assert_eq!(
            source.cmap().unwrap().map_codepoint(0xe001_u32),
            decoded.cmap().unwrap().map_codepoint(0xe001_u32)
        );
        assert_eq!(
            source.hhea().unwrap().ascender(),
            decoded.hhea().unwrap().ascender()
        );
        assert_eq!(
            source.name().unwrap().offset_data().as_bytes(),
            decoded.name().unwrap().offset_data().as_bytes()
        );
    }

    fn assert_directory(output: &[u8], tables: &SerializedFontTables) {
        let count = u16::from_be_bytes(output[12..14].try_into().unwrap()) as usize;
        let mut offset = HEADER_SIZE;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let flags = output[offset];
            offset += 1;
            let index = usize::from(flags & 0x3f);
            let tag = if index == 63 {
                let tag = output[offset..offset + 4].try_into().unwrap();
                offset += 4;
                tag
            } else {
                KNOWN_TAGS[index]
            };
            let length = read_base128(output, &mut offset);
            entries.push((tag, flags >> 6, length));
        }

        let mut expected = tables
            .tables()
            .iter()
            .filter(|table| table.tag != *b"DSIG")
            .collect::<Vec<_>>();
        expected.sort_unstable_by_key(|table| table.tag);
        move_loca_after_glyf(&mut expected);
        assert_eq!(
            entries,
            expected
                .iter()
                .map(|table| {
                    (
                        table.tag,
                        if matches!(&table.tag, b"glyf" | b"loca") {
                            3
                        } else {
                            0
                        },
                        table.bytes.len() as u32,
                    )
                })
                .collect::<Vec<_>>()
        );

        let compressed_size = u32::from_be_bytes(output[20..24].try_into().unwrap()) as usize;
        let stream_end = offset + compressed_size;
        assert!(stream_end <= output.len());
        assert!(output.len() - stream_end < 4);
        assert!(output[stream_end..].iter().all(|byte| *byte == 0));
        let expected_sfnt_size = 12
            + 16 * expected.len()
            + expected
                .iter()
                .map(|table| (table.bytes.len() + 3) & !3)
                .sum::<usize>();
        assert_eq!(
            u32::from_be_bytes(output[16..20].try_into().unwrap()) as usize,
            expected_sfnt_size
        );
    }

    fn read_base128(input: &[u8], offset: &mut usize) -> u32 {
        let mut value = 0_u32;
        loop {
            let byte = input[*offset];
            *offset += 1;
            value = (value << 7) | u32::from(byte & 0x7f);
            if byte & 0x80 == 0 {
                return value;
            }
        }
    }

    fn sfnt_table<'a>(sfnt: &'a [u8], wanted: &[u8; 4]) -> Option<&'a [u8]> {
        let count = u16::from_be_bytes(sfnt[4..6].try_into().ok()?) as usize;
        (0..count).find_map(|index| {
            let entry = 12 + index * 16;
            if &sfnt[entry..entry + 4] != wanted {
                return None;
            }
            let offset = u32::from_be_bytes(sfnt[entry + 8..entry + 12].try_into().ok()?) as usize;
            let length = u32::from_be_bytes(sfnt[entry + 12..entry + 16].try_into().ok()?) as usize;
            sfnt.get(offset..offset + length)
        })
    }
}
