use std::io::{Error, ErrorKind, Write};

use flate2::write::ZlibEncoder;
use flate2::Compression;

const WOFF_HEADER_SIZE: usize = 44;
const META_OFFSET_POS: usize = 24;
const META_LENGTH_POS: usize = 28;
const META_ORIG_LENGTH_POS: usize = 32;
const LENGTH_POS: usize = 8;

pub(crate) fn ttf_to_woff1(ttf: &[u8], metadata: Option<&str>) -> Result<Vec<u8>, Error> {
    let mut woff_buf = ::woff::version1::compress(ttf, 1, 0)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF compression failed"))?;
    if let Some(metadata) = metadata {
        inject_woff_metadata(&mut woff_buf, metadata)?;
    }
    Ok(woff_buf)
}

pub(crate) fn ttf_to_woff2(ttf: &[u8]) -> Result<Vec<u8>, Error> {
    ::woff::version2::compress(ttf, "", 11, true)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 compression failed"))
}

fn inject_woff_metadata(woff: &mut Vec<u8>, metadata: &str) -> Result<(), Error> {
    if woff.len() < WOFF_HEADER_SIZE {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "WOFF buffer is too short to contain a valid header.",
        ));
    }

    let meta_raw = metadata.as_bytes();
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(meta_raw)?;
    let meta_compressed = encoder.finish()?;

    let meta_offset = woff.len() as u32;
    let meta_length = meta_compressed.len() as u32;
    let meta_orig_length = meta_raw.len() as u32;

    woff.extend_from_slice(&meta_compressed);

    let total_length = woff.len() as u32;

    woff[LENGTH_POS..LENGTH_POS + 4].copy_from_slice(&total_length.to_be_bytes());
    woff[META_OFFSET_POS..META_OFFSET_POS + 4].copy_from_slice(&meta_offset.to_be_bytes());
    woff[META_LENGTH_POS..META_LENGTH_POS + 4].copy_from_slice(&meta_length.to_be_bytes());
    woff[META_ORIG_LENGTH_POS..META_ORIG_LENGTH_POS + 4]
        .copy_from_slice(&meta_orig_length.to_be_bytes());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_metadata_into_woff_header() {
        let mut woff = vec![0u8; WOFF_HEADER_SIZE];

        inject_woff_metadata(&mut woff, "<metadata />").unwrap();

        assert!(woff.len() > WOFF_HEADER_SIZE);

        let total_length = u32::from_be_bytes(woff[LENGTH_POS..LENGTH_POS + 4].try_into().unwrap());
        let meta_offset = u32::from_be_bytes(
            woff[META_OFFSET_POS..META_OFFSET_POS + 4]
                .try_into()
                .unwrap(),
        );
        let meta_length = u32::from_be_bytes(
            woff[META_LENGTH_POS..META_LENGTH_POS + 4]
                .try_into()
                .unwrap(),
        );
        let meta_orig = u32::from_be_bytes(
            woff[META_ORIG_LENGTH_POS..META_ORIG_LENGTH_POS + 4]
                .try_into()
                .unwrap(),
        );

        assert_eq!(total_length, woff.len() as u32);
        assert_eq!(meta_offset, WOFF_HEADER_SIZE as u32);
        assert_eq!(meta_orig, 12);
        assert!(meta_length > 0);
    }

    #[test]
    fn rejects_buffer_too_short() {
        let mut woff = vec![0u8; 10];
        let err = inject_woff_metadata(&mut woff, "test").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }
}
