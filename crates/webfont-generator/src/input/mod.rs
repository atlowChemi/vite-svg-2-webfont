mod files;
mod options;
mod sources;

#[cfg(test)]
pub(crate) use options::GenerationOptions;

#[cfg(test)]
pub(crate) use files::load_variant_svg_files;
pub(crate) use files::{load_svg_files_with_hooks, load_variant_svg_files_with_hooks};
#[cfg(any(test, feature = "bench"))]
pub(crate) use options::resolve_generate_webfonts_options;
pub(crate) use options::{
    ResolvedFileGenerationOptions, ResolvedGenerationOptions, ResolvedVariants,
    default_output_dest, finalize_generate_webfonts_options,
    resolve_validated_generate_webfonts_options, validate_color_names,
};
pub(crate) use sources::{
    LoadedSvgSource, VariantFamilySources, VariantGlyphSource, build_variant_family_sources,
    resolve_missing_glyphs, validate_glyph_names,
};
