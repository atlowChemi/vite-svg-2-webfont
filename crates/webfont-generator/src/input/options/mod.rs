mod common;
#[cfg(all(test, feature = "fs"))]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::io::{Error, ErrorKind};
#[cfg(feature = "fs")]
use std::path::Path;

use super::LoadedSvgSource;
use crate::types::color::ResolvedColorSelection;
use crate::types::{FontType, FormatOptions, MissingGlyphBehavior};
#[cfg(feature = "fs")]
use crate::types::{FontVariant, GenerateWebfontsOptions};

pub(crate) use common::VariantMetadata;
pub(crate) use common::{GenerationOptions, ResolvedGenerationOptions};

#[cfg(feature = "fs")]
struct FileGenerationOptions {
    common: GenerationOptions,
    cache_location: Option<String>,
    css_dest: Option<String>,
    css_template: Option<String>,
    dest: String,
    files: Vec<String>,
    html_dest: Option<String>,
    html_template: Option<String>,
    variants: Option<Vec<FontVariant>>,
    write_files: Option<bool>,
}

#[cfg(feature = "fs")]
impl From<GenerateWebfontsOptions> for FileGenerationOptions {
    fn from(options: GenerateWebfontsOptions) -> Self {
        let GenerateWebfontsOptions {
            color_glyphs,
            color_slots,
            cache_location,
            ascent,
            center_horizontally,
            center_vertically,
            css,
            css_dest,
            css_template,
            codepoints,
            css_fonts_url,
            descent,
            dest,
            files,
            fixed_width,
            format_options,
            html,
            html_dest,
            html_template,
            incremental,
            font_height,
            font_name,
            font_style,
            font_weight,
            ligature,
            missing_glyphs,
            normalize,
            order,
            optimize_output,
            preserve_aspect_ratio,
            round,
            start_codepoint,
            template_options,
            types,
            variant_class_prefix,
            variants,
            write_files,
        } = options;
        Self {
            cache_location,
            common: GenerationOptions {
                color_glyphs,
                color_slots,
                ascent,
                center_horizontally,
                center_vertically,
                css,
                codepoints,
                css_fonts_url,
                descent,
                fixed_width,
                format_options,
                html,
                incremental,
                font_height,
                font_name,
                font_style,
                font_weight,
                ligature,
                missing_glyphs,
                normalize,
                order,
                optimize_output,
                preserve_aspect_ratio,
                round,
                start_codepoint,
                template_options,
                types,
                variant_class_prefix,
            },
            css_dest,
            css_template,
            dest,
            files,
            html_dest,
            html_template,
            variants,
            write_files,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ResolvedVariants {
    pub variants: Vec<ResolvedFontVariant>,
    pub default_index: usize,
}

#[derive(Clone)]
pub(crate) struct ResolvedFontVariant {
    pub name: String,
    pub source_ids: Vec<String>,
    pub weight: u16,
    pub class_name: String,
    pub selector: String,
}

#[derive(Clone)]
#[cfg(feature = "fs")]
pub(crate) struct ResolvedFileGenerationOptions {
    pub common: ResolvedGenerationOptions,
    pub cache_location: Option<String>,
    pub css_dest: String,
    pub css_template: Option<String>,
    pub dest: String,
    pub files: Vec<String>,
    pub html_dest: String,
    pub html_template: Option<String>,
    pub variants: Option<ResolvedVariants>,
    pub write_files: bool,
}

// The file adapter extends shared settings; existing internal field readers keep borrowing
// the same settings without cloning them. Content-only consumers accept the common type.
#[cfg(feature = "fs")]
impl std::ops::Deref for ResolvedFileGenerationOptions {
    type Target = ResolvedGenerationOptions;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

#[cfg(feature = "fs")]
impl std::ops::DerefMut for ResolvedFileGenerationOptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}

const DEFAULT_FONT_TYPES: [FontType; 3] = [FontType::Eot, FontType::Woff, FontType::Woff2];

fn validate_color_formats(
    selection: Option<&ResolvedColorSelection>,
    types: &[FontType],
) -> std::io::Result<()> {
    let active = selection.is_some_and(|selection| match selection {
        ResolvedColorSelection::All => true,
        ResolvedColorSelection::Named(names) => !names.is_empty(),
    });
    if active
        && types
            .iter()
            .any(|kind| matches!(kind, FontType::Svg | FontType::Eot))
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "options.colorGlyphs: incompatible output formats: {}. Use TTF, WOFF, or WOFF2.",
                types
                    .iter()
                    .filter(|kind| matches!(kind, FontType::Svg | FontType::Eot))
                    .map(|kind| kind.as_extension())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    Ok(())
}

const DEFAULT_FONT_ORDER: [FontType; 5] = [
    FontType::Eot,
    FontType::Woff2,
    FontType::Woff,
    FontType::Ttf,
    FontType::Svg,
];

#[cfg(feature = "fs")]
fn validate_file_options(options: &FileGenerationOptions) -> std::io::Result<()> {
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
            options.common.validate_without_variants()?;
        }
        Some(variants) => {
            if !options.files.is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.files\" must be empty when \"options.variants\" is provided.",
                ));
            }
            validate_variants(&options.common, &file_variant_metadata(variants), "files")?;
        }
    }

    if options.common.css.unwrap_or(true)
        && let Some(ref path) = options.css_template
        && !Path::new(path).exists()
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("\"options.cssTemplate\" file not found: {path}"),
        ));
    }

    if options.common.html.unwrap_or(false)
        && let Some(ref path) = options.html_template
        && !Path::new(path).exists()
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("\"options.htmlTemplate\" file not found: {path}"),
        ));
    }

    validate_format_options(options.common.format_options.as_ref())
}

pub(crate) fn validate_format_options(options: Option<&FormatOptions>) -> std::io::Result<()> {
    if let Some(quality) = options
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

#[cfg(feature = "fs")]
fn file_variant_metadata(variants: &[FontVariant]) -> Vec<VariantMetadata<'_>> {
    variants
        .iter()
        .map(|variant| VariantMetadata {
            name: &variant.name,
            weight: variant.weight,
            default: variant.default,
            source_count: variant.files.len(),
        })
        .collect()
}

pub(crate) fn validate_variants(
    options: &GenerationOptions,
    variants: &[VariantMetadata<'_>],
    membership_field: &str,
) -> std::io::Result<()> {
    if variants.len() < 2 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.variants\" must contain at least two variants.",
        ));
    }
    if options
        .types
        .as_ref()
        .is_some_and(|types| types.contains(&FontType::Svg) || types.contains(&FontType::Eot))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "\"options.types\" cannot include \"svg\" or \"eot\" with \"options.variants\".",
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
        if variant.source_count == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("\"{path}.{membership_field}\" is empty."),
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
    variants: &[VariantMetadata<'_>],
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

#[cfg(feature = "fs")]
fn resolve_variants(
    variants: &[FontVariant],
    class_prefix: &str,
) -> std::io::Result<ResolvedVariants> {
    resolve_variant_metadata(
        &file_variant_metadata(variants),
        variants
            .iter()
            .map(|variant| variant.files.clone())
            .collect(),
        class_prefix,
    )
}

pub(crate) fn resolve_variant_metadata(
    variants: &[VariantMetadata<'_>],
    memberships: Vec<Vec<String>>,
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
    let mut resolved = Vec::with_capacity(variants.len());

    for ((variant, weight), source_ids) in variants.iter().zip(weights).zip(memberships) {
        let class_name = format!("{class_prefix}{}", variant.name);
        let selector = serialize_css_identifier(&class_name);
        resolved.push(ResolvedFontVariant {
            name: variant.name.to_owned(),
            source_ids,
            weight,
            class_name,
            selector,
        });
    }

    Ok(ResolvedVariants {
        variants: resolved,
        default_index,
    })
}

#[cfg(feature = "fs")]
pub(crate) fn resolve_validated_generate_webfonts_options(
    options: GenerateWebfontsOptions,
) -> std::io::Result<ResolvedFileGenerationOptions> {
    let options = FileGenerationOptions::from(options);
    validate_file_options(&options)?;
    resolve_file_options(options)
}

#[cfg(any(test, feature = "bench"))]
#[cfg(feature = "fs")]
pub(crate) fn resolve_generate_webfonts_options(
    options: GenerateWebfontsOptions,
) -> std::io::Result<ResolvedFileGenerationOptions> {
    resolve_file_options(options.into())
}

#[cfg(feature = "fs")]
fn resolve_file_options(
    options: FileGenerationOptions,
) -> std::io::Result<ResolvedFileGenerationOptions> {
    let mut settings = options.common;
    let class_prefix = settings.variant_class_prefix.take();
    let common = settings.resolve(options.variants.is_some())?;
    let css_dest = options
        .css_dest
        .unwrap_or_else(|| default_output_dest(&options.dest, &common.font_name, "css"));
    let html_dest = options
        .html_dest
        .unwrap_or_else(|| default_output_dest(&options.dest, &common.font_name, "html"));
    let write_files = options.write_files.unwrap_or(true);
    let variants = options
        .variants
        .as_deref()
        .map(|variants| resolve_variants(variants, class_prefix.as_deref().unwrap_or("icon--")))
        .transpose()?;

    if options.cache_location.as_deref() == Some("") {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "\"options.cacheLocation\" must not be empty.".to_owned(),
        ));
    }
    Ok(ResolvedFileGenerationOptions {
        common,
        cache_location: options.cache_location,
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
        dest: options.dest,
        files: options.files,
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
        variants,
        write_files,
    })
}

pub(crate) fn finalize_generate_webfonts_options(
    options: &mut ResolvedGenerationOptions,
    source_files: &[LoadedSvgSource],
) -> std::io::Result<()> {
    validate_color_names(
        options,
        source_files.iter().map(|file| file.glyph_name.as_str()),
    )?;
    options.codepoints = resolve_codepoints(
        source_files
            .iter()
            .map(|source_file| source_file.glyph_name.as_str()),
        &options.explicit_codepoints,
        options.start_codepoint,
    )?;

    Ok(())
}

pub(crate) fn validate_color_names<'a>(
    options: &ResolvedGenerationOptions,
    names: impl Iterator<Item = &'a str>,
) -> std::io::Result<()> {
    if let Some(ResolvedColorSelection::Named(selected)) = &options.color_selection {
        let names: HashSet<_> = names.collect();
        let missing: Vec<_> = selected
            .iter()
            .filter(|name| !names.contains(name.as_str()))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "options.colorGlyphs: unknown glyph names: {}",
                    missing.join(", ")
                ),
            ));
        }
    }
    Ok(())
}

fn resolved_font_types(types: Option<&[FontType]>, variants: bool) -> Vec<FontType> {
    match types {
        Some(types) => types.to_vec(),
        None if variants => vec![FontType::Woff, FontType::Woff2],
        None => DEFAULT_FONT_TYPES.to_vec(),
    }
}

fn resolve_font_type_order(order: Option<&[FontType]>, types: &[FontType]) -> Vec<FontType> {
    match order {
        Some(order) => order.to_vec(),
        None => DEFAULT_FONT_ORDER
            .iter()
            .copied()
            .filter(|font_type| types.contains(font_type))
            .collect(),
    }
}

#[cfg(feature = "fs")]
pub(crate) fn default_output_dest(dest: &str, font_name: &str, extension: &str) -> String {
    Path::new(dest)
        .join(format!("{font_name}.{extension}"))
        .to_string_lossy()
        .into_owned()
}

fn validate_font_type_order(
    order: Option<&[FontType]>,
    requested_types: &[FontType],
) -> std::io::Result<()> {
    if let Some(order) = order
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

pub(super) fn resolve_codepoints<'a>(
    names: impl IntoIterator<Item = &'a str>,
    codepoints: &BTreeMap<String, u32>,
    start_codepoint: u32,
) -> Result<BTreeMap<String, u32>, Error> {
    let mut resolved_codepoints = codepoints.clone();
    let mut used_codepoints: HashSet<u32> = resolved_codepoints.values().copied().collect();
    let mut next_codepoint = start_codepoint;

    for name in names {
        if resolved_codepoints.contains_key(name) {
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

        resolved_codepoints.insert(name.to_owned(), next_codepoint);
        used_codepoints.insert(next_codepoint);
        next_codepoint = next_codepoint.saturating_add(1);
    }

    Ok(resolved_codepoints)
}
