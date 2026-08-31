use std::io::ErrorKind;

use super::EOT_PREFIX_SIZE;

use super::EOT_VERSION;

use super::tables_to_eot;
use crate::byte_helpers::BigEndian;
use crate::sfnt::SerializedFontTables;
use crate::test_helpers::fixture_font_tables;

fn name_table(records: &[(u16, u16, u16, u16, &str)]) -> Vec<u8> {
    let string_offset = 6 + records.len() * 12;
    let mut table = Vec::with_capacity(string_offset);
    table.extend_from_slice(&0_u16.to_be_bytes());
    table.extend_from_slice(&(records.len() as u16).to_be_bytes());
    table.extend_from_slice(&(string_offset as u16).to_be_bytes());

    let mut strings = Vec::new();
    for &(platform, encoding, language, name_id, value) in records {
        let encoded = value
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>();
        table.extend_from_slice(&platform.to_be_bytes());
        table.extend_from_slice(&encoding.to_be_bytes());
        table.extend_from_slice(&language.to_be_bytes());
        table.extend_from_slice(&name_id.to_be_bytes());
        table.extend_from_slice(&(encoded.len() as u16).to_be_bytes());
        table.extend_from_slice(&(strings.len() as u16).to_be_bytes());
        strings.extend_from_slice(&encoded);
    }
    table.extend_from_slice(&strings);
    table
}

fn font_tables(os2: Vec<u8>, head: Vec<u8>, name: Vec<u8>) -> SerializedFontTables {
    SerializedFontTables::new(vec![(*b"OS/2", os2), (*b"head", head), (*b"name", name)])
        .expect("test tables should serialize")
}

fn valid_os2() -> Vec<u8> {
    let mut os2 = vec![0; 86];
    os2[32..42].copy_from_slice(&[2, 11, 6, 4, 2, 2, 2, 2, 2, 4]);
    {
        let mut writer = BigEndian::new(&mut os2);
        writer.write_u16_at(0, 1);
        writer.write_u16_at(4, 700);
        for (index, value) in [1, 2, 4, 8].into_iter().enumerate() {
            writer.write_u32_at(42 + index * 4, value);
        }
        writer.write_u16_at(62, 1);
        writer.write_u32_at(78, 0x1122_3344);
        writer.write_u32_at(82, 0x5566_7788);
    }
    os2
}

fn eot_name(value: &str) -> Vec<u8> {
    let encoded = value
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    let mut result = (encoded.len() as u16).to_le_bytes().to_vec();
    result.extend_from_slice(&encoded);
    result.extend_from_slice(&[0, 0]);
    result
}

#[test]
fn generates_an_eot_buffer_with_expected_header() {
    let result = tables_to_eot(&fixture_font_tables()).expect("expected eot generation to succeed");

    assert_eq!(&result[34..36], b"LP");
    assert_eq!(
        u32::from_le_bytes(result[8..12].try_into().unwrap()),
        EOT_VERSION
    );
}

#[test]
fn copies_ttf_metadata_and_english_windows_names() {
    let tables = font_tables(
        valid_os2(),
        vec![0; 12],
        name_table(&[
            (3, 1, 0x0409, 1, "Family"),
            (3, 1, 0x0409, 2, "Bold"),
            (3, 1, 0x0409, 4, "Family Bold"),
            (3, 1, 0x0409, 5, "Version 1"),
            (3, 1, 0x0409, 99, "Ignored ID"),
            (3, 1, 0x0411, 1, "Ignored language"),
        ]),
    );
    let ttf = tables.ttf().to_vec();
    let checksum_adjustment = tables
        .tables()
        .iter()
        .find(|table| table.tag == *b"head")
        .expect("head table should exist")
        .bytes[8..12]
        .try_into()
        .map(u32::from_be_bytes)
        .expect("checksum adjustment should be four bytes");

    let result = tables_to_eot(&tables).expect("expected EOT generation to succeed");

    assert_eq!(
        u32::from_le_bytes(result[0..4].try_into().unwrap()) as usize,
        result.len()
    );
    assert_eq!(
        u32::from_le_bytes(result[4..8].try_into().unwrap()) as usize,
        ttf.len()
    );
    assert_eq!(&result[16..26], &[2, 11, 6, 4, 2, 2, 2, 2, 2, 4]);
    assert_eq!(result[26], 1);
    assert_eq!(result[27], 1);
    assert_eq!(u32::from_le_bytes(result[28..32].try_into().unwrap()), 700);
    for (index, value) in [1, 2, 4, 8].into_iter().enumerate() {
        assert_eq!(
            u32::from_le_bytes(result[36 + index * 4..40 + index * 4].try_into().unwrap()),
            value
        );
    }
    assert_eq!(&result[52..56], &0x1122_3344_u32.to_le_bytes());
    assert_eq!(&result[56..60], &0x5566_7788_u32.to_le_bytes());
    assert_eq!(&result[60..64], &checksum_adjustment.to_le_bytes());

    let mut suffix = Vec::new();
    for value in ["Family", "Bold", "Version 1", "Family Bold"] {
        suffix.extend_from_slice(&eot_name(value));
    }
    suffix.extend_from_slice(&[0, 0]);
    suffix.extend_from_slice(&ttf);
    assert_eq!(&result[EOT_PREFIX_SIZE..], suffix);
}

#[test]
fn rejects_missing_or_malformed_required_tables() {
    let missing = SerializedFontTables::new(vec![]).unwrap();
    let error = tables_to_eot(&missing).expect_err("missing tables should fail");
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(error.to_string().contains("Required TTF sections"));

    let cases = [
        (
            font_tables(vec![0; 63], vec![0; 12], name_table(&[])),
            ErrorKind::UnexpectedEof,
            "reading u16",
        ),
        (
            font_tables(valid_os2(), vec![0; 11], name_table(&[])),
            ErrorKind::UnexpectedEof,
            "reading u32",
        ),
        (
            font_tables(valid_os2(), vec![0; 12], vec![0; 5]),
            ErrorKind::UnexpectedEof,
            "reading u16",
        ),
    ];
    for (tables, kind, message) in cases {
        let error = tables_to_eot(&tables).expect_err("malformed table should fail");
        assert_eq!(error.kind(), kind);
        assert!(error.to_string().contains(message));
    }
}

#[test]
fn rejects_invalid_name_record_ranges_and_utf16() {
    let mut outside = name_table(&[(3, 1, 0x0409, 1, "A")]);
    outside.truncate(18);
    let error = tables_to_eot(&font_tables(valid_os2(), vec![0; 12], outside))
        .expect_err("out-of-range name should fail");
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(error.to_string().contains("Malformed name table record"));

    let mut odd = name_table(&[(3, 1, 0x0409, 1, "A")]);
    odd[14..16].copy_from_slice(&1_u16.to_be_bytes());
    let error = tables_to_eot(&font_tables(valid_os2(), vec![0; 12], odd))
        .expect_err("odd UTF-16BE name should fail");
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(error.to_string().contains("Malformed UTF-16BE"));
}
