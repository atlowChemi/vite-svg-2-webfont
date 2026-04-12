//! [Web Open Font Format][1] of version 1.0.
//!
//! [1]: https://www.w3.org/TR/WOFF/

mod ffi;

/// Compress.
pub fn compress(data: &[u8], major_version: usize, minor_version: usize) -> Option<Vec<u8>> {
    let mut size = 0;
    let mut status = 0;
    let data = unsafe {
        ffi::woffEncode(
            data.as_ptr() as _,
            data.len() as _,
            major_version as _,
            minor_version as _,
            &mut size,
            &mut status,
        )
    };
    finalize(data, size, status)
}

/// Decompress.
pub fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    let mut size = 0;
    let mut status = 0;
    let data =
        unsafe { ffi::woffDecode(data.as_ptr() as _, data.len() as _, &mut size, &mut status) };
    finalize(data, size, status)
}

fn finalize(data: *const u8, size: u32, status: u32) -> Option<Vec<u8>> {
    debug_assert_eq!(status & 0xFF, 0);
    if !data.is_null() && status & 0xFF == 0 {
        let mut buffer = Vec::with_capacity(size as _);
        unsafe {
            std::ptr::copy_nonoverlapping(data, buffer.as_mut_ptr(), size as _);
            buffer.set_len(size as _);
            libc::free(data as *mut _);
        }
        Some(buffer)
    } else if !data.is_null() {
        unsafe {
            libc::free(data as *mut _);
        }
        None
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{read, write};

    const DEFAULT_MAJOR: usize = 1;
    const DEFAULT_MINOR: usize = 0;

    macro_rules! ok(($result:expr) => ($result.unwrap()));

    #[test]
    fn otf() {
        ok!(write(
            "tests/fixtures/Roboto-Regular.otf.woff",
            ok!(super::compress(
                &ok!(read("tests/fixtures/Roboto-Regular.otf")),
                DEFAULT_MAJOR,
                DEFAULT_MINOR,
            )),
        ));
        ok!(write(
            "tests/fixtures/Roboto-Regular.otf",
            ok!(super::decompress(&ok!(read(
                "tests/fixtures/Roboto-Regular.otf.woff"
            )))),
        ));
    }

    #[test]
    fn ttf() {
        ok!(write(
            "tests/fixtures/Roboto-Regular.ttf.woff",
            ok!(super::compress(
                &ok!(read("tests/fixtures/Roboto-Regular.ttf")),
                DEFAULT_MAJOR,
                DEFAULT_MINOR,
            )),
        ));
        ok!(write(
            "tests/fixtures/Roboto-Regular.ttf",
            ok!(super::decompress(&ok!(read(
                "tests/fixtures/Roboto-Regular.ttf.woff"
            )))),
        ));
    }
}
