use super::*;
use crate::test_helpers::fixture_font_tables;
use flate2::read::ZlibDecoder;
use std::io::Read;

#[test]
fn cached_and_uncached_encoders_inject_metadata() {
    let tables = fixture_font_tables();
    let metadata = "<metadata />";
    let mut cache = Woff1PayloadCache::default();

    for woff in [
        tables_to_woff1(&tables, Some(metadata)).unwrap(),
        tables_to_woff1_cached(&tables, Some(metadata), &mut cache).unwrap(),
    ] {
        let total_length = u32::from_be_bytes(woff[LENGTH_POS..LENGTH_POS + 4].try_into().unwrap());
        let meta_offset = u32::from_be_bytes(
            woff[META_OFFSET_POS..META_OFFSET_POS + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let meta_length = u32::from_be_bytes(
            woff[META_LENGTH_POS..META_LENGTH_POS + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let meta_orig = u32::from_be_bytes(
            woff[META_ORIG_LENGTH_POS..META_ORIG_LENGTH_POS + 4]
                .try_into()
                .unwrap(),
        );
        let mut decoded = String::new();
        ZlibDecoder::new(&woff[meta_offset..meta_offset + meta_length])
            .read_to_string(&mut decoded)
            .unwrap();

        assert_eq!(total_length, woff.len() as u32);
        assert_eq!(meta_offset % 4, 0);
        assert_eq!(meta_orig, metadata.len() as u32);
        assert_eq!(decoded, metadata);
    }
}

#[test]
fn rejects_buffer_too_short() {
    let mut woff = vec![0u8; 10];
    let err = inject_woff_metadata(&mut woff, "test").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
}

#[test]
fn woff1_tables_round_trip_to_sfnt_tables() {
    let tables = fixture_font_tables();
    let woff = tables_to_woff1(&tables, None).expect("expected woff generation to succeed");

    assert_eq!(&woff[0..4], b"wOFF");
    assert_eq!(
        u16::from_be_bytes(woff[12..14].try_into().unwrap()) as usize,
        tables.tables().len()
    );

    for index in 0..tables.tables().len() {
        let entry_offset = WOFF_HEADER_SIZE + index * WOFF_TABLE_ENTRY_SIZE;
        let tag: [u8; 4] = woff[entry_offset..entry_offset + 4].try_into().unwrap();
        let table = tables
            .tables()
            .iter()
            .find(|table| table.tag == tag)
            .expect("expected WOFF table tag to exist in SFNT");
        let offset =
            u32::from_be_bytes(woff[entry_offset + 4..entry_offset + 8].try_into().unwrap())
                as usize;
        let comp_len = u32::from_be_bytes(
            woff[entry_offset + 8..entry_offset + 12]
                .try_into()
                .unwrap(),
        ) as usize;
        let orig_len = u32::from_be_bytes(
            woff[entry_offset + 12..entry_offset + 16]
                .try_into()
                .unwrap(),
        ) as usize;
        let payload = &woff[offset..offset + comp_len];
        let decoded = if comp_len < orig_len {
            let mut decoded = Vec::new();
            ZlibDecoder::new(payload)
                .read_to_end(&mut decoded)
                .expect("expected table payload to decompress");
            decoded
        } else {
            payload.to_vec()
        };

        assert_eq!(decoded, table.bytes);
    }
}
