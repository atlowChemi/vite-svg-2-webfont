mod builder;
mod serialize;

#[cfg(test)]
pub(crate) use builder::TtfOptions;
pub(crate) use builder::{
    CachedCompiledGlyph, Woff1PayloadCache, Woff2TransformCache, Woff2TransformPayload, build,
    ttf_options_from_options,
};
pub(crate) use serialize::{SerializedFontTables, SerializedTable};
