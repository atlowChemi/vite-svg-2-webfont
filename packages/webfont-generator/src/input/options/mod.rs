#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::io::{Error, ErrorKind};
use std::path::Path;

use super::files::LoadedSvgFile;
use crate::types::{FontType, FormatOptions, GenerateWebfontsOptions};

#[derive(Clone)]
pub(crate) struct ResolvedGenerateWebfontsOptions {
    pub ascent: Option<f64>,
    pub center_horizontally: Option<bool>,
    pub center_vertically: Option<bool>,
    pub css: bool,
    pub css_dest: String,
    pub css_template: Option<String>,
    /// Fully-resolved codepoints for the current glyph set (explicit + auto-assigned). Rebuilt
    /// by `finalize_generate_webfonts_options` from `explicit_codepoints` whenever the set changes.
    pub codepoints: BTreeMap<String, u32>,
    /// The user-supplied codepoints, kept as the stable base so re-resolving after an
    /// incremental add/remove assigns the same codepoints a fresh build would.
    pub explicit_codepoints: BTreeMap<String, u32>,
    pub css_fonts_url: Option<String>,
    pub descent: Option<f64>,
    pub dest: String,
    pub files: Vec<String>,
    pub fixed_width: Option<bool>,
    pub format_options: Option<FormatOptions>,
    pub html: bool,
    pub incremental: bool,
    pub html_dest: String,
    pub html_template: Option<String>,
    pub font_height: Option<f64>,
    pub font_name: String,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub ligature: bool,
    pub normalize: bool,
    pub order: Vec<FontType>,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
    pub round: Option<f64>,
    pub start_codepoint: u32,
    pub template_options: Option<serde_json::Map<String, serde_json::Value>>,
    pub types: Vec<FontType>,
    pub write_files: bool,
}

const DEFAULT_FONT_TYPES: [FontType; 3] = [FontType::Eot, FontType::Woff, FontType::Woff2];

const DEFAULT_FONT_ORDER: [FontType; 5] = [
    FontType::Eot,
    FontType::Woff2,
    FontType::Woff,
    FontType::Ttf,
    FontType::Svg,
];

pub(crate) fn validate_generate_webfonts_options(
    options: &GenerateWebfontsOptions,
) -> std::io::Result<()> {
    if options.dest.is_empty() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "\"options.dest\" is empty.".to_owned(),
        ));
    }

    if options.files.is_empty() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "\"options.files\" is empty.".to_owned(),
        ));
    }

    if options.css.unwrap_or(true)
        && let Some(ref path) = options.css_template
        && !Path::new(path).exists()
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("\"options.cssTemplate\" file not found: {path}"),
        ));
    }

    if options.html.unwrap_or(false)
        && let Some(ref path) = options.html_template
        && !Path::new(path).exists()
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("\"options.htmlTemplate\" file not found: {path}"),
        ));
    }

    if let Some(quality) = options
        .format_options
        .as_ref()
        .and_then(|value| value.woff2.as_ref())
        .and_then(|value| value.compression_quality)
        && quality > 11
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "\"options.formatOptions.woff2.compressionQuality\" must be between 0 and 11, got {quality}."
            ),
        ));
    }

    Ok(())
}

pub(crate) fn resolve_generate_webfonts_options(
    options: GenerateWebfontsOptions,
) -> std::io::Result<ResolvedGenerateWebfontsOptions> {
    let types = resolved_font_types(&options);
    validate_font_type_order(&options, &types)?;
    let order = resolve_font_type_order(&options, &types);
    let css = options.css.unwrap_or(true);
    let html = options.html.unwrap_or(false);
    let font_name = options.font_name.unwrap_or_else(|| "iconfont".to_owned());
    let css_dest = options
        .css_dest
        .unwrap_or_else(|| default_output_dest(&options.dest, &font_name, "css"));
    let html_dest = options
        .html_dest
        .unwrap_or_else(|| default_output_dest(&options.dest, &font_name, "html"));
    let write_files = options.write_files.unwrap_or(true);
    let explicit_codepoints: BTreeMap<String, u32> =
        options.codepoints.unwrap_or_default().into_iter().collect();

    let svg_format = options
        .format_options
        .as_ref()
        .and_then(|fo| fo.svg.as_ref());
    let center_vertically = svg_format
        .and_then(|s| s.center_vertically)
        .or(options.center_vertically);
    let optimize_output = svg_format
        .and_then(|s| s.optimize_output)
        .or(options.optimize_output);
    let preserve_aspect_ratio = svg_format
        .and_then(|s| s.preserve_aspect_ratio)
        .or(options.preserve_aspect_ratio);

    Ok(ResolvedGenerateWebfontsOptions {
        ascent: options.ascent,
        center_horizontally: options.center_horizontally,
        center_vertically,
        css,
        css_dest,
        css_template: match options.css_template {
            Some(ref t) if t.is_empty() => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.cssTemplate\" must not be empty.".to_owned(),
                ));
            }
            other => other,
        },
        codepoints: explicit_codepoints.clone(),
        explicit_codepoints,
        css_fonts_url: options.css_fonts_url,
        descent: options.descent,
        dest: options.dest,
        files: options.files,
        fixed_width: options.fixed_width,
        format_options: options.format_options,
        html,
        html_dest,
        html_template: match options.html_template {
            Some(ref t) if t.is_empty() => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.htmlTemplate\" must not be empty.".to_owned(),
                ));
            }
            other => other,
        },
        incremental: options.incremental.unwrap_or(false),
        font_height: options.font_height,
        font_name,
        font_style: options.font_style,
        font_weight: options.font_weight,
        ligature: options.ligature.unwrap_or(true),
        normalize: options.normalize.unwrap_or(true),
        order,
        optimize_output,
        preserve_aspect_ratio,
        round: options.round,
        start_codepoint: options.start_codepoint.unwrap_or(0xF101),
        template_options: options.template_options,
        types,
        write_files,
    })
}

pub(crate) fn finalize_generate_webfonts_options(
    options: &mut ResolvedGenerateWebfontsOptions,
    source_files: &[LoadedSvgFile],
) -> std::io::Result<()> {
    options.codepoints = resolve_codepoints(
        source_files,
        &options.explicit_codepoints,
        options.start_codepoint,
    )?;

    Ok(())
}

fn resolved_font_types(options: &GenerateWebfontsOptions) -> Vec<FontType> {
    match &options.types {
        Some(types) => types.clone(),
        None => DEFAULT_FONT_TYPES.to_vec(),
    }
}

fn resolve_font_type_order(options: &GenerateWebfontsOptions, types: &[FontType]) -> Vec<FontType> {
    match &options.order {
        Some(order) => order.clone(),
        None => DEFAULT_FONT_ORDER
            .iter()
            .copied()
            .filter(|font_type| types.contains(font_type))
            .collect(),
    }
}

pub(crate) fn default_output_dest(dest: &str, font_name: &str, extension: &str) -> String {
    Path::new(dest)
        .join(format!("{font_name}.{extension}"))
        .to_string_lossy()
        .into_owned()
}

fn validate_font_type_order(
    options: &GenerateWebfontsOptions,
    requested_types: &[FontType],
) -> std::io::Result<()> {
    if let Some(order) = &options.order
        && let Some(invalid_type) = order
            .iter()
            .copied()
            .find(|font_type| !requested_types.contains(font_type))
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Invalid font type order: '{}' is not present in 'types'.",
                invalid_type.as_extension()
            ),
        ));
    }

    Ok(())
}

fn resolve_codepoints(
    source_files: &[LoadedSvgFile],
    codepoints: &BTreeMap<String, u32>,
    start_codepoint: u32,
) -> Result<BTreeMap<String, u32>, Error> {
    let mut resolved_codepoints = codepoints.clone();
    let mut used_codepoints: HashSet<u32> = resolved_codepoints.values().copied().collect();
    let mut next_codepoint = start_codepoint;

    for source_file in source_files {
        let name = source_file.glyph_name.clone();

        if resolved_codepoints.contains_key(&name) {
            continue;
        }

        while used_codepoints.contains(&next_codepoint) {
            next_codepoint += 1;
        }

        resolved_codepoints.insert(name, next_codepoint);
        used_codepoints.insert(next_codepoint);
        next_codepoint += 1;
    }

    Ok(resolved_codepoints)
}
