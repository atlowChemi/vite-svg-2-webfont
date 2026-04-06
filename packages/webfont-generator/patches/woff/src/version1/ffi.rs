unsafe extern "C" {
    pub fn woffEncode(
        sfntData: *const u8,
        sfntLen: u32,
        majorVersion: u16,
        minorVersion: u16,
        woffLen: *mut u32,
        status: *mut u32,
    ) -> *const u8;

    pub fn woffDecode(
        woffData: *const u8,
        woffLen: u32,
        sfntLen: *mut u32,
        status: *mut u32,
    ) -> *const u8;
}
