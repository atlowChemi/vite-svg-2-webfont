use std::io::{Error, ErrorKind};

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

const SFNT_NUM_TABLES_OFFSET: usize = 4;
const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_ENTRY_SIZE: usize = 16;
const SFNT_TABLE_TAG_OFFSET: usize = 0;
const SFNT_TABLE_OFFSET_OFFSET: usize = 8;
const SFNT_TABLE_LENGTH_OFFSET: usize = 12;

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

pub(crate) fn ttf_to_eot(ttf: &[u8]) -> Result<Vec<u8>, Error> {
    let mut prefix = vec![0_u8; EOT_PREFIX_SIZE];
    write_u32_le(&mut prefix, EOT_FONT_LENGTH_OFFSET, ttf.len() as u32)?;
    write_u32_le(&mut prefix, EOT_VERSION_OFFSET, EOT_VERSION)?;
    prefix[EOT_CHARSET_OFFSET] = EOT_CHARSET;
    write_u16_le(&mut prefix, EOT_MAGIC_OFFSET, EOT_MAGIC)?;

    let mut family_name = vec![0_u8];
    let mut subfamily_name = vec![0_u8];
    let mut full_name = vec![0_u8];
    let mut version_string = vec![0_u8];

    let mut have_os2 = false;
    let mut have_name = false;
    let mut have_head = false;
    let num_tables = read_u16_be(ttf, SFNT_NUM_TABLES_OFFSET)? as usize;

    for table_index in 0..num_tables {
        let entry_offset = SFNT_HEADER_SIZE + table_index * SFNT_TABLE_ENTRY_SIZE;
        let tag = ttf
            .get(entry_offset + SFNT_TABLE_TAG_OFFSET..entry_offset + SFNT_TABLE_TAG_OFFSET + 4)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Malformed TTF table directory."))?;
        let table_offset = read_u32_be(ttf, entry_offset + SFNT_TABLE_OFFSET_OFFSET)? as usize;
        let table_length = read_u32_be(ttf, entry_offset + SFNT_TABLE_LENGTH_OFFSET)? as usize;
        let table = ttf
            .get(table_offset..table_offset + table_length)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Malformed TTF table slice."))?;

        if tag == b"OS/2" {
            have_os2 = true;
            let os2_version = read_u16_be(table, 0)?;

            prefix[EOT_FONT_PANOSE_OFFSET..EOT_FONT_PANOSE_OFFSET + 10].copy_from_slice(
                table
                    .get(OS2_PANOSE_OFFSET..OS2_PANOSE_OFFSET + 10)
                    .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Malformed OS/2 table."))?,
            );
            prefix[EOT_ITALIC_OFFSET] = (read_u16_be(table, OS2_FS_SELECTION_OFFSET)? & 0x01) as u8;
            write_u32_le(
                &mut prefix,
                EOT_WEIGHT_OFFSET,
                u32::from(read_u16_be(table, OS2_WEIGHT_OFFSET)?),
            )?;
            for range_index in 0..4 {
                write_u32_le(
                    &mut prefix,
                    EOT_UNICODE_RANGE_OFFSET + range_index * 4,
                    read_u32_be(table, OS2_UNICODE_RANGE_OFFSET + range_index * 4)?,
                )?;
            }
            if os2_version >= 1 {
                for range_index in 0..2 {
                    write_u32_le(
                        &mut prefix,
                        EOT_CODEPAGE_RANGE_OFFSET + range_index * 4,
                        read_u32_be(table, OS2_CODEPAGE_RANGE_OFFSET + range_index * 4)?,
                    )?;
                }
            }
        } else if tag == b"head" {
            have_head = true;
            write_u32_le(
                &mut prefix,
                EOT_CHECKSUM_ADJUSTMENT_OFFSET,
                read_u32_be(table, HEAD_CHECKSUM_ADJUSTMENT_OFFSET)?,
            )?;
        } else if tag == b"name" {
            have_name = true;
            let name_count = read_u16_be(table, NAME_TABLE_COUNT_OFFSET)? as usize;
            let string_offset = read_u16_be(table, NAME_TABLE_STRING_OFFSET_OFFSET)? as usize;

            for record_index in 0..name_count {
                let record_offset = NAME_TABLE_HEADER_SIZE + record_index * NAME_RECORD_SIZE;
                let platform_id = read_u16_be(table, record_offset + NAME_PLATFORM_ID_OFFSET)?;
                let encoding_id = read_u16_be(table, record_offset + NAME_ENCODING_ID_OFFSET)?;
                let language_id = read_u16_be(table, record_offset + NAME_LANGUAGE_ID_OFFSET)?;
                let name_id = read_u16_be(table, record_offset + NAME_NAME_ID_OFFSET)?;
                let name_length = read_u16_be(table, record_offset + NAME_LENGTH_OFFSET)? as usize;
                let name_offset = read_u16_be(table, record_offset + NAME_OFFSET_OFFSET)? as usize;

                if platform_id == 3 && encoding_id == 1 && language_id == LANGUAGE_ENGLISH {
                    let value = table
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
        }

        if have_os2 && have_name && have_head {
            break;
        }
    }

    if !(have_os2 && have_name && have_head) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Required TTF sections not found for EOT conversion.",
        ));
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
    write_u32_le(&mut eot, EOT_LENGTH_OFFSET, eot_length)?;

    Ok(eot)
}

fn strbuf(utf16be: &[u8]) -> Result<Vec<u8>, Error> {
    if !utf16be.len().is_multiple_of(2) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Malformed UTF-16BE name record.",
        ));
    }
    let mut output = vec![0_u8; utf16be.len() + 4];
    write_u16_le(&mut output, 0, utf16be.len() as u16)?;

    for (index, chunk) in utf16be.chunks_exact(2).enumerate() {
        output[2 + index * 2] = chunk[1];
        output[2 + index * 2 + 1] = chunk[0];
    }

    Ok(output)
}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16, Error> {
    let bytes = data.get(offset..offset + 2).ok_or_else(|| {
        Error::new(
            ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading u16.",
        )
    })?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data.get(offset..offset + 4).ok_or_else(|| {
        Error::new(
            ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading u32.",
        )
    })?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn write_u16_le(data: &mut [u8], offset: usize, value: u16) -> Result<(), Error> {
    let bytes = value.to_le_bytes();
    let slice = data.get_mut(offset..offset + 2).ok_or_else(|| {
        Error::new(
            ErrorKind::UnexpectedEof,
            "Unexpected EOF while writing u16.",
        )
    })?;
    slice.copy_from_slice(&bytes);
    Ok(())
}

fn write_u32_le(data: &mut [u8], offset: usize, value: u32) -> Result<(), Error> {
    let bytes = value.to_le_bytes();
    let slice = data.get_mut(offset..offset + 4).ok_or_else(|| {
        Error::new(
            ErrorKind::UnexpectedEof,
            "Unexpected EOF while writing u32.",
        )
    })?;
    slice.copy_from_slice(&bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ttf_to_eot, EOT_VERSION};
    use crate::{ttf::generate_ttf_font_bytes, GenerateWebfontsOptions};

    #[test]
    fn generates_an_eot_buffer_with_expected_header() {
        let ttf_result = generate_ttf_font_bytes(GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_string(),
            files: vec![format!(
                "{}/../vite-svg-2-webfont/src/fixtures/webfont-test/svg/add.svg",
                env!("CARGO_MANIFEST_DIR")
            )],
            html: Some(false),
            font_name: Some("iconfont".to_string()),
            ligature: Some(false),
            ..Default::default()
        })
        .expect("expected ttf generation to succeed");

        let result = ttf_to_eot(&ttf_result).expect("expected eot generation to succeed");

        assert_eq!(&result[34..36], b"LP");
        assert_eq!(
            u32::from_le_bytes(result[8..12].try_into().unwrap()),
            EOT_VERSION
        );
    }
}
