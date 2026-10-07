mod files;
mod options;

#[cfg(test)]
pub(crate) use options::resolve_generate_webfonts_options_with_color;

#[cfg(test)]
pub(crate) use files::load_variant_svg_files;
pub(crate) use files::{
    LoadedSvgFile, VariantFamilySources, VariantGlyphSource, build_variant_family_sources,
    resolve_missing_glyphs, validate_glyph_names,
};
pub(crate) use files::{load_svg_files_with_hooks, load_variant_svg_files_with_hooks};
pub(crate) use options::{
    ResolvedGenerateWebfontsOptions, ResolvedVariants, default_output_dest,
    finalize_generate_webfonts_options, resolve_generate_webfonts_options,
    validate_generate_webfonts_options,
};
