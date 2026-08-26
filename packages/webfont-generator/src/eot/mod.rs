use std::io::{Error, ErrorKind};

use crate::byte_helpers::{BigEndian, LittleEndian};
use crate::sfnt::SerializedFontTables;

const EOT_PREFIX_SIZE: usize = 82;
const EOT_VERSION: u32 = 0x0002_0001;
const EOT_MAGIC: u16 = 0x504c;
const EOT_CHARSET: u8 = 1;
const LANGUAGE_ENGLISH: u16 = 0x0409;

const EOT_LENGTH_OFFSET: usize = 0;
const EOT_FONT_LENGTH_OFFSET: usize = 4;
const EOT_VERSION_OFFSET: usize = 8;
const EOT_FONT_PANOSE_OFFSET: usize = 16;
const EOT_CHARSET_OFFSET: usize = 26;
const EOT_ITALIC_OFFSET: usize = 27;
const EOT_WEIGHT_OFFSET: usize = 28;
const EOT_MAGIC_OFFSET: usize = 34;
const EOT_UNICODE_RANGE_OFFSET: usize = 36;
const EOT_CODEPAGE_RANGE_OFFSET: usize = 52;
const EOT_CHECKSUM_ADJUSTMENT_OFFSET: usize = 60;

const OS2_WEIGHT_OFFSET: usize = 4;
const OS2_PANOSE_OFFSET: usize = 32;
const OS2_UNICODE_RANGE_OFFSET: usize = 42;
const OS2_FS_SELECTION_OFFSET: usize = 62;
const OS2_CODEPAGE_RANGE_OFFSET: usize = 78;

const HEAD_CHECKSUM_ADJUSTMENT_OFFSET: usize = 8;

const NAME_TABLE_COUNT_OFFSET: usize = 2;
const NAME_TABLE_STRING_OFFSET_OFFSET: usize = 4;
const NAME_TABLE_HEADER_SIZE: usize = 6;
const NAME_RECORD_SIZE: usize = 12;
const NAME_PLATFORM_ID_OFFSET: usize = 0;
const NAME_ENCODING_ID_OFFSET: usize = 2;
const NAME_LANGUAGE_ID_OFFSET: usize = 4;
const NAME_NAME_ID_OFFSET: usize = 6;
const NAME_LENGTH_OFFSET: usize = 8;
const NAME_OFFSET_OFFSET: usize = 10;

pub(crate) fn tables_to_eot(tables: &SerializedFontTables) -> Result<Vec<u8>, Error> {
    let ttf = tables.ttf();
    let mut os2 = None;
    let mut head = None;
    let mut name = None;
    for table in tables.tables() {
        match &table.tag {
            b"OS/2" => os2 = Some(table.bytes.as_slice()),
            b"head" => head = Some(table.bytes.as_slice()),
            b"name" => name = Some(table.bytes.as_slice()),
            _ => {}
        }
    }
    let (Some(os2), Some(head), Some(name)) = (os2, head, name) else {
        return Err(missing_required_tables());
    };

    let mut prefix = vec![0_u8; EOT_PREFIX_SIZE];
    {
        let mut writer = LittleEndian::new(&mut prefix);
        writer.write_u32_at(EOT_FONT_LENGTH_OFFSET, ttf.len() as u32);
        writer.write_u32_at(EOT_VERSION_OFFSET, EOT_VERSION);
        writer.write_u16_at(EOT_MAGIC_OFFSET, EOT_MAGIC);
    }
    prefix[EOT_CHARSET_OFFSET] = EOT_CHARSET;

    let mut family_name = vec![0_u8];
    let mut subfamily_name = vec![0_u8];
    let mut full_name = vec![0_u8];
    let mut version_string = vec![0_u8];

    let os2_reader = BigEndian::new(os2);
    let os2_version = os2_reader.read_u16(0)?;
    prefix[EOT_FONT_PANOSE_OFFSET..EOT_FONT_PANOSE_OFFSET + 10].copy_from_slice(
        os2.get(OS2_PANOSE_OFFSET..OS2_PANOSE_OFFSET + 10)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Malformed OS/2 table."))?,
    );
    prefix[EOT_ITALIC_OFFSET] = (os2_reader.read_u16(OS2_FS_SELECTION_OFFSET)? & 0x01) as u8;
    {
        let mut writer = LittleEndian::new(&mut prefix);
        writer.write_u32_at(
            EOT_WEIGHT_OFFSET,
            u32::from(os2_reader.read_u16(OS2_WEIGHT_OFFSET)?),
        );
        for range_index in 0..4 {
            writer.write_u32_at(
                EOT_UNICODE_RANGE_OFFSET + range_index * 4,
                os2_reader.read_u32(OS2_UNICODE_RANGE_OFFSET + range_index * 4)?,
            );
        }
        if os2_version >= 1 {
            for range_index in 0..2 {
                writer.write_u32_at(
                    EOT_CODEPAGE_RANGE_OFFSET + range_index * 4,
                    os2_reader.read_u32(OS2_CODEPAGE_RANGE_OFFSET + range_index * 4)?,
                );
            }
        }
        writer.write_u32_at(
            EOT_CHECKSUM_ADJUSTMENT_OFFSET,
            BigEndian::new(head).read_u32(HEAD_CHECKSUM_ADJUSTMENT_OFFSET)?,
        );
    }

    let name_reader = BigEndian::new(name);
    let name_count = name_reader.read_u16(NAME_TABLE_COUNT_OFFSET)? as usize;
    let string_offset = name_reader.read_u16(NAME_TABLE_STRING_OFFSET_OFFSET)? as usize;

    for record_index in 0..name_count {
        let record_offset = NAME_TABLE_HEADER_SIZE + record_index * NAME_RECORD_SIZE;
        let platform_id = name_reader.read_u16(record_offset + NAME_PLATFORM_ID_OFFSET)?;
        let encoding_id = name_reader.read_u16(record_offset + NAME_ENCODING_ID_OFFSET)?;
        let language_id = name_reader.read_u16(record_offset + NAME_LANGUAGE_ID_OFFSET)?;
        let name_id = name_reader.read_u16(record_offset + NAME_NAME_ID_OFFSET)?;
        let name_length = name_reader.read_u16(record_offset + NAME_LENGTH_OFFSET)? as usize;
        let name_offset = name_reader.read_u16(record_offset + NAME_OFFSET_OFFSET)? as usize;

        if platform_id == 3 && encoding_id == 1 && language_id == LANGUAGE_ENGLISH {
            let value = name
                .get(string_offset + name_offset..string_offset + name_offset + name_length)
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "Malformed name table record.")
                })?;
            let encoded = strbuf(value)?;

            match name_id {
                1 => family_name = encoded,
                2 => subfamily_name = encoded,
                4 => full_name = encoded,
                5 => version_string = encoded,
                _ => {}
            }
        }
    }

    let mut eot = Vec::with_capacity(
        prefix.len()
            + family_name.len()
            + subfamily_name.len()
            + version_string.len()
            + full_name.len()
            + 2
            + ttf.len(),
    );
    eot.extend_from_slice(&prefix);
    eot.extend_from_slice(&family_name);
    eot.extend_from_slice(&subfamily_name);
    eot.extend_from_slice(&version_string);
    eot.extend_from_slice(&full_name);
    eot.extend_from_slice(&[0, 0]);
    eot.extend_from_slice(ttf);
    let eot_length = eot.len() as u32;
    LittleEndian::new(&mut eot).write_u32_at(EOT_LENGTH_OFFSET, eot_length);

    Ok(eot)
}

fn missing_required_tables() -> Error {
    Error::new(
        ErrorKind::InvalidData,
        "Required TTF sections not found for EOT conversion.",
    )
}

fn strbuf(utf16be: &[u8]) -> Result<Vec<u8>, Error> {
    if !utf16be.len().is_multiple_of(2) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Malformed UTF-16BE name record.",
        ));
    }
    let mut output = vec![0_u8; utf16be.len() + 4];
    LittleEndian::new(&mut output).write_u16_at(0, utf16be.len() as u16);

    for (index, chunk) in utf16be.as_chunks::<2>().0.iter().enumerate() {
        output[2 + index * 2] = chunk[1];
        output[2 + index * 2 + 1] = chunk[0];
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::{EOT_PREFIX_SIZE, EOT_VERSION, tables_to_eot};
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
        let result =
            tables_to_eot(&fixture_font_tables()).expect("expected eot generation to succeed");

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
}
