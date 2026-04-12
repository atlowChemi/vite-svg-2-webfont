//! [Web Open Font Format][1] of version 2.0.
//!
//! [1]: https://www.w3.org/TR/WOFF2/

mod ffi;

/// Compress.
pub fn compress<T>(data: &[u8], metadata: T, quality: usize, transform: bool) -> Option<Vec<u8>>
where
    T: Into<Vec<u8>>,
{
    let metadata = match std::ffi::CString::new(metadata) {
        Ok(metadata) => metadata,
        _ => return None,
    };
    let metadata_size = metadata.count_bytes();
    let size = unsafe {
        ffi::ComputeTTFToWOFF2Size(
            data.as_ptr() as *const _,
            data.len(),
            metadata.as_ptr() as *const _,
            metadata_size,
        )
    };
    let mut buffer = vec![0; size];
    let mut size = buffer.len();
    let status = unsafe {
        ffi::ConvertTTFToWOFF2(
            data.as_ptr() as *const _,
            data.len(),
            buffer.as_mut_ptr() as *mut _,
            &mut size as *mut _,
            metadata.as_ptr() as *const _,
            metadata_size,
            quality as core::ffi::c_int,
            transform as core::ffi::c_int,
        )
    };
    debug_assert_ne!(status, 0);
    if status == 0 {
        return None;
    }
    buffer.truncate(size);
    buffer.into()
}

/// Decompress.
pub fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    let size = unsafe { ffi::ComputeWOFF2ToTTFSize(data.as_ptr() as *const _, data.len()) };
    let mut buffer = vec![0; size];
    let status = unsafe {
        ffi::ConvertWOFF2ToTTF(
            buffer.as_mut_ptr() as *mut _,
            size,
            data.as_ptr() as *const _,
            data.len(),
        )
    };
    debug_assert_ne!(status, 0);
    if status == 0 {
        return None;
    }
    buffer.into()
}

#[cfg(test)]
mod tests {
    use std::fs::{read, write};

    const DEFAULT_METADATA: &str = "";
    const DEFAULT_QUALITY: usize = 8;
    const DEFAULT_TRANSFORM: bool = true;

    macro_rules! ok(($result:expr) => ($result.unwrap()));

    #[test]
    fn otf() {
        ok!(write(
            "tests/fixtures/Roboto-Regular.otf.woff2",
            ok!(super::compress(
                &ok!(read("tests/fixtures/Roboto-Regular.otf")),
                DEFAULT_METADATA,
                DEFAULT_QUALITY,
                DEFAULT_TRANSFORM,
            )),
        ));
        ok!(write(
            "tests/fixtures/Roboto-Regular.otf",
            ok!(super::decompress(&ok!(read(
                "tests/fixtures/Roboto-Regular.otf.woff2"
            )))),
        ));
    }

    #[test]
    fn ttf() {
        ok!(write(
            "tests/fixtures/Roboto-Regular.ttf.woff2",
            ok!(super::compress(
                &ok!(read("tests/fixtures/Roboto-Regular.ttf")),
                DEFAULT_METADATA,
                DEFAULT_QUALITY,
                DEFAULT_TRANSFORM,
            )),
        ));
        ok!(write(
            "tests/fixtures/Roboto-Regular.ttf",
            ok!(super::decompress(&ok!(read(
                "tests/fixtures/Roboto-Regular.ttf.woff2"
            )))),
        ));
    }
}
