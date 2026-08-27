mod files;
mod options;

#[cfg(feature = "napi")]
pub(crate) use files::load_svg_files_napi;
pub(crate) use files::{LoadedSvgFile, load_svg_files, validate_glyph_names};
pub(crate) use options::{
    default_output_dest, finalize_generate_webfonts_options, resolve_generate_webfonts_options,
    validate_generate_webfonts_options,
};
