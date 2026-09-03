#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::io::{Error, ErrorKind};
use std::path::Path;

use super::files::LoadedSvgFile;
use crate::types::{
    FontType, FontVariant, FormatOptions, GenerateWebfontsOptions, MissingGlyphBehavior,
};

#[derive(Clone)]
#[allow(
    dead_code,
    reason = "variant metadata is consumed by later generation phases"
)]
pub(crate) struct ResolvedVariants {
    pub variants: Vec<ResolvedFontVariant>,
    pub default_index: usize,
}

#[derive(Clone)]
#[allow(
    dead_code,
    reason = "variant metadata is consumed by later generation phases"
)]
pub(crate) struct ResolvedFontVariant {
    pub name: String,
    pub files: Vec<String>,
    pub weight: u16,
    pub class_name: String,
    pub selector: String,
    pub filename_component: String,
}

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
    #[allow(
        dead_code,
        reason = "variant metadata is consumed by later generation phases"
    )]
    pub variants: Option<ResolvedVariants>,
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

    match options.variants.as_deref() {
        None => {
            if options.files.is_empty() {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.files\" is empty.".to_owned(),
                ));
            }
            if options.missing_glyphs.is_some() {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.missingGlyphs\" requires \"options.variants\".",
                ));
            }
            if options.variant_class_prefix.is_some() {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.variantClassPrefix\" requires \"options.variants\".",
                ));
            }
        }
        Some(variants) => validate_variants(options, variants)?,
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

fn validate_variants(
    options: &GenerateWebfontsOptions,
    variants: &[crate::types::FontVariant],
) -> std::io::Result<()> {
    if !options.files.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.files\" must be empty when \"options.variants\" is provided.",
        ));
    }
    if variants.len() < 2 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.variants\" must contain at least two variants.",
        ));
    }
    if options
        .types
        .as_ref()
        .is_some_and(|types| types.contains(&FontType::Svg))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.types\" cannot include \"svg\" with \"options.variants\".",
        ));
    }
    if options.incremental == Some(true) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.incremental\" cannot be true with \"options.variants\".",
        ));
    }
    if options.font_weight.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.fontWeight\" cannot be used with \"options.variants\".",
        ));
    }
    if options
        .template_options
        .as_ref()
        .is_some_and(|template| template.contains_key("variantClassPrefix"))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.templateOptions.variantClassPrefix\" cannot be used with \"options.variants\"; use \"options.variantClassPrefix\".",
        ));
    }

    let class_prefix = options.variant_class_prefix.as_deref().unwrap_or("icon--");
    if class_prefix.is_empty()
        || class_prefix.contains(char::is_whitespace)
        || class_prefix.contains('\0')
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.variantClassPrefix\" must be non-empty and contain neither whitespace nor NUL.",
        ));
    }

    let mut names = HashSet::with_capacity(variants.len());
    let mut default_count = 0;
    for (index, variant) in variants.iter().enumerate() {
        let path = format!("options.variants[{index}]");
        if variant.files.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("\"{path}.files\" is empty."),
            ));
        }
        if variant.name.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("\"{path}.name\" is empty."),
            ));
        }
        if variant.name.contains(char::is_whitespace) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("\"{path}.name\" contains whitespace."),
            ));
        }
        if variant.name.contains('\0') {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("\"{path}.name\" contains NUL."),
            ));
        }
        if !names.insert(&variant.name) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "\"{path}.name\" duplicates variant name \"{}\".",
                    variant.name
                ),
            ));
        }
        if let Some(weight) = variant.weight
            && !(1..=1000).contains(&weight)
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("\"{path}.weight\" must be between 1 and 1000, got {weight}."),
            ));
        }
        default_count += usize::from(variant.default == Some(true));
    }

    if default_count != 1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "\"options.variants\" must contain exactly one default variant, found {default_count}."
            ),
        ));
    }

    if variants.iter().all(|variant| variant.weight.is_some()) {
        for (index, pair) in variants.windows(2).enumerate() {
            if pair[0].weight >= pair[1].weight {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "\"options.variants[{}].weight\" must be greater than the preceding explicit weight.",
                        index + 1,
                    ),
                ));
            }
        }
    }

    if let Some(missing) = &options.missing_glyphs {
        match missing.behavior {
            MissingGlyphBehavior::Fallback => {
                let Some(fallback) = missing.variant.as_deref() else {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "\"options.missingGlyphs.variant\" is required when behavior is \"fallback\".",
                    ));
                };
                if !variants.iter().any(|variant| variant.name == fallback) {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "\"options.missingGlyphs.variant\" does not name a configured variant: \"{fallback}\"."
                        ),
                    ));
                }
            }
            MissingGlyphBehavior::Blank | MissingGlyphBehavior::Error
                if missing.variant.is_some() =>
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.missingGlyphs.variant\" is only valid when behavior is \"fallback\".",
                ));
            }
            MissingGlyphBehavior::Blank | MissingGlyphBehavior::Error => {}
        }
    }

    Ok(())
}

fn serialize_css_identifier(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut escaped = String::with_capacity(value.len());
    for (index, character) in characters.iter().copied().enumerate() {
        let codepoint = character as u32;
        if character == '\0' {
            escaped.push('\u{fffd}');
        } else if (1..=0x1f).contains(&codepoint)
            || codepoint == 0x7f
            || (index == 0 && character.is_ascii_digit())
            || (index == 1 && character.is_ascii_digit() && characters.first() == Some(&'-'))
        {
            escaped.push('\\');
            escaped.push_str(&format!("{codepoint:x} "));
        } else if index == 0 && character == '-' && characters.len() == 1 {
            escaped.push_str("\\-");
        } else if codepoint >= 0x80
            || character == '-'
            || character == '_'
            || character.is_ascii_alphanumeric()
        {
            escaped.push(character);
        } else {
            escaped.push('\\');
            escaped.push(character);
        }
    }
    escaped
}

fn resolve_variant_weights(
    variants: &[FontVariant],
    default_index: usize,
) -> std::io::Result<Vec<u16>> {
    let mut weights = variants
        .iter()
        .map(|variant| variant.weight)
        .collect::<Vec<_>>();
    weights[default_index].get_or_insert(400);

    let mut anchors = Vec::with_capacity(variants.len() + 2);
    anchors.push((None, 0_u32));
    anchors.extend(
        weights
            .iter()
            .enumerate()
            .filter_map(|(index, weight)| weight.map(|weight| (Some(index), u32::from(weight)))),
    );
    anchors.push((None, 1001));

    for pair in anchors.windows(2) {
        let (left_index, left_weight) = pair[0];
        let (right_index, right_weight) = pair[1];
        if left_weight >= right_weight {
            let index = right_index.unwrap_or(variants.len() - 1);
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "\"options.variants[{index}].weight\" must be greater than the preceding resolved weight {left_weight}."
                ),
            ));
        }

        let start = left_index.map_or(0, |index| index + 1);
        let end = right_index.unwrap_or(variants.len());
        let count = end - start;
        if count == 0 {
            continue;
        }
        if right_weight - left_weight - 1 < count as u32 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "\"options.variants[{start}..{}].weight\" cannot fit {count} automatic weights between {left_weight} and {right_weight}.",
                    end - 1,
                ),
            ));
        }

        let before_default = right_index.is_some_and(|index| index <= default_index);
        let step_weights = (0..count)
            .map(|offset| {
                if before_default {
                    i64::from(right_weight) - 100 * (count - offset) as i64
                } else {
                    i64::from(left_weight) + 100 * (offset + 1) as i64
                }
            })
            .collect::<Vec<_>>();
        let step_weights_fit = step_weights
            .iter()
            .all(|weight| *weight > i64::from(left_weight) && *weight < i64::from(right_weight));

        for offset in 0..count {
            let weight = if step_weights_fit {
                step_weights[offset] as u32
            } else {
                left_weight
                    + (right_weight - left_weight) * (offset as u32 + 1) / (count as u32 + 1)
            };
            weights[start + offset] = Some(weight as u16);
        }
    }

    Ok(weights.into_iter().map(Option::unwrap).collect())
}

fn encode_filename_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("~{byte:02X}"));
        }
    }

    let upper = encoded.to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            upper.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
        });
    if reserved {
        format!("~{:02X}{}", encoded.as_bytes()[0], &encoded[1..])
    } else {
        encoded
    }
}

fn resolve_variants(
    variants: &[FontVariant],
    class_prefix: &str,
) -> std::io::Result<ResolvedVariants> {
    let default_index = variants
        .iter()
        .position(|variant| variant.default == Some(true))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "\"options.variants\" must contain a default variant.",
            )
        })?;
    let weights = resolve_variant_weights(variants, default_index)?;
    let mut filenames = HashSet::with_capacity(variants.len());
    let mut resolved = Vec::with_capacity(variants.len());

    for (index, (variant, weight)) in variants.iter().zip(weights).enumerate() {
        let filename_component = encode_filename_component(&variant.name);
        if !filenames.insert(filename_component.to_ascii_lowercase()) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "\"options.variants[{index}].name\" produces a duplicate case-insensitive filename component \"{filename_component}\"."
                ),
            ));
        }
        let class_name = format!("{class_prefix}{}", variant.name);
        let selector = serialize_css_identifier(&class_name);
        resolved.push(ResolvedFontVariant {
            name: variant.name.clone(),
            files: variant.files.clone(),
            weight,
            class_name,
            selector,
            filename_component,
        });
    }

    Ok(ResolvedVariants {
        variants: resolved,
        default_index,
    })
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
    let variants = options
        .variants
        .as_deref()
        .map(|variants| {
            resolve_variants(
                variants,
                options.variant_class_prefix.as_deref().unwrap_or("icon--"),
            )
        })
        .transpose()?;

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
        variants,
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
            next_codepoint = next_codepoint.checked_add(1).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "Unable to assign another glyph codepoint: the u32 range is exhausted.",
                )
            })?;
        }

        resolved_codepoints.insert(name, next_codepoint);
        used_codepoints.insert(next_codepoint);
        next_codepoint = next_codepoint.saturating_add(1);
    }

    Ok(resolved_codepoints)
}
