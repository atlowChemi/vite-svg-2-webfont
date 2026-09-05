mod builder;
mod serialize;

#[cfg(test)]
pub(crate) use builder::TtfOptions;
pub(crate) use builder::{CachedCompiledGlyph, build, ttf_options_from_options};
pub(crate) use builder::{build_static_variant, build_variant};
pub(crate) use serialize::{SerializedFontTables, SerializedTable};
