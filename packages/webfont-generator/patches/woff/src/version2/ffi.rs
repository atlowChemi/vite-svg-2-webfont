unsafe extern "C" {
    pub fn ComputeTTFToWOFF2Size(
        data: *const u8,
        length: libc::size_t,
        extended_metadata: *const core::ffi::c_char,
        extended_metadata_length: libc::size_t,
    ) -> libc::size_t;

    pub fn ComputeWOFF2ToTTFSize(data: *const u8, length: libc::size_t) -> libc::size_t;

    pub fn ConvertTTFToWOFF2(
        data: *const u8,
        length: libc::size_t,
        result: *mut u8,
        result_length: *mut libc::size_t,
        extended_metadata: *const core::ffi::c_char,
        extended_metadata_length: libc::size_t,
        brotli_quality: core::ffi::c_int,
        allow_transforms: core::ffi::c_int,
    ) -> core::ffi::c_int;

    pub fn ConvertWOFF2ToTTF(
        result: *mut u8,
        result_length: libc::size_t,
        data: *const u8,
        length: libc::size_t,
    ) -> core::ffi::c_int;
}
