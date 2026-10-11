mod builder;
mod serialize;

#[cfg(test)]
pub(crate) use builder::TtfOptions;
pub(crate) use builder::{CachedCompiledGlyph, build, build_variant, ttf_options_from_options};
pub(crate) use serialize::{SerializedFontTables, SerializedTable};
