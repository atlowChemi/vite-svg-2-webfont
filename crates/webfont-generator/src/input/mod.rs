#[cfg(feature = "fs")]
mod files;
mod options;
mod sources;

#[cfg(any(feature = "in_memory", test))]
pub(crate) use options::GenerationOptions;
#[cfg(feature = "in_memory")]
pub(crate) use options::{
    VariantMetadata, resolve_variant_metadata, validate_format_options, validate_variants,
};

#[cfg(all(test, feature = "fs"))]
pub(crate) use files::load_variant_svg_files;
#[cfg(feature = "fs")]
pub(crate) use files::{load_svg_files_with_hooks, load_variant_svg_files_with_hooks};
#[cfg(all(feature = "fs", any(test, feature = "bench")))]
pub(crate) use options::resolve_generate_webfonts_options;
#[cfg(feature = "fs")]
pub(crate) use options::{
    ResolvedFileGenerationOptions, default_output_dest, resolve_validated_generate_webfonts_options,
};
pub(crate) use options::{
    ResolvedGenerationOptions, ResolvedVariants, finalize_generate_webfonts_options,
    validate_color_names,
};
pub(crate) use sources::{
    LoadedSvgSource, VariantFamilySources, VariantGlyphSource, build_variant_family_sources,
    resolve_missing_glyphs, validate_glyph_names,
};
