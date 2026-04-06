//! Converter for Web Open Font Format.

// Ensure libz-sys is linked (provides zlib for the vendored C code).
#[cfg(feature = "version1")]
use libz_sys as _;

#[cfg(feature = "version1")]
pub mod version1;
#[cfg(feature = "version2")]
pub mod version2;
