mod builder;
mod serialize;

#[cfg(test)]
pub(crate) use builder::TtfOptions;
pub(crate) use builder::{CachedCompiledGlyph, build, ttf_options_from_options};
#[allow(
    unused_imports,
    reason = "variant SFNT output is connected to public formats in a later phase"
)]
pub(crate) use builder::{VariantFontBuild, build_variant};
pub(crate) use serialize::{SerializedFontTables, SerializedTable};
