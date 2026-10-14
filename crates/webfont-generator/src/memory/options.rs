use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::sync::Arc;

use crate::input::{
    GenerationOptions, LoadedSvgSource, ResolvedGenerationOptions, ResolvedVariants,
    VariantMetadata, resolve_variant_metadata, validate_format_options, validate_glyph_names,
    validate_variants,
};
use crate::{ColorGlyphSelection, FontType, FormatOptions, MissingGlyphOptions};

/// Named SVG text. `id` is an opaque, nonempty cache key, never a file path.
#[derive(Clone)]
pub struct SvgSource {
    /// Stable identity. Repeated IDs across variants must have identical names and text.
    pub id: String,
    /// Glyph name: nonempty Unicode letters/numbers, hyphens or underscores.
    /// This subset is safe in the engine's built-in SVG/CSS/HTML renderers.
    pub name: String,
    /// UTF-8 SVG document. External image paths are not loaded.
    pub svg: String,
}

/// One ordered design in a text-input variant family.
#[derive(Clone)]
pub struct SourceFontVariant {
    /// Unique variant name; follows the file API's variant naming rules.
    pub name: String,
    /// Ordered sources. IDs and glyph names must be unique within this design.
    pub sources: Vec<SvgSource>,
    /// Optional weight in 1..=1000; omitted weights use shared interpolation rules.
    pub weight: Option<u16>,
    /// Exactly one variant must set this to `Some(true)`.
    pub default: Option<bool>,
}

/// Filesystem-free generation settings. Defaults match the file API's `GenerateWebfontsOptions`.
/// Templates are text (`Some("")` renders empty), not paths. No output is written.
#[derive(Clone, Default)]
pub struct GenerateWebfontsFromSourcesOptions {
    /// Ordered sources for ordinary generation; empty when `variants` is supplied.
    pub sources: Vec<SvgSource>,
    /// Ordered variant designs, using the same font encoders as file generation.
    pub variants: Option<Vec<SourceFontVariant>>,
    /// Optional CSS Handlebars source; compiled on first CSS render.
    pub css_template: Option<String>,
    /// Optional HTML Handlebars source; compiled on first HTML render.
    pub html_template: Option<String>,
    /// Color glyph selection; supports TTF, WOFF and WOFF2.
    pub color_glyphs: Option<ColorGlyphSelection>,
    /// Font ascent.
    pub ascent: Option<f64>,
    /// Center glyphs horizontally.
    pub center_horizontally: Option<bool>,
    /// Center glyphs vertically.
    pub center_vertically: Option<bool>,
    /// Include CSS in collected outputs (default true).
    pub css: Option<bool>,
    /// Explicit glyph name to Unicode assignments.
    pub codepoints: Option<HashMap<String, u32>>,
    /// URL prefix; absent means relative font filenames in both CSS and HTML.
    pub css_fonts_url: Option<String>,
    /// Font descent.
    pub descent: Option<f64>,
    /// Use a common advance width.
    pub fixed_width: Option<bool>,
    /// Format-specific encoding settings.
    pub format_options: Option<FormatOptions>,
    /// Include HTML in collected outputs (default false).
    pub html: Option<bool>,
    /// Retain geometry and encoder caches for subsequent source updates.
    pub incremental: Option<bool>,
    /// Output font height.
    pub font_height: Option<f64>,
    /// Logical font name (default `iconfont`).
    pub font_name: Option<String>,
    /// Font style.
    pub font_style: Option<String>,
    /// Ordinary font weight; incompatible with variants.
    pub font_weight: Option<String>,
    /// Enable name ligatures (default true).
    pub ligature: Option<bool>,
    /// Variant missing-glyph policy.
    pub missing_glyphs: Option<MissingGlyphOptions>,
    /// Normalize glyph dimensions (default true).
    pub normalize: Option<bool>,
    /// CSS font format preference order.
    pub order: Option<Vec<FontType>>,
    /// Optimize SVG output paths.
    pub optimize_output: Option<bool>,
    /// Preserve source aspect ratios.
    pub preserve_aspect_ratio: Option<bool>,
    /// Coordinate rounding precision.
    pub round: Option<f64>,
    /// First automatic codepoint (default U+F101).
    pub start_codepoint: Option<u32>,
    /// Additional template context values.
    pub template_options: Option<serde_json::Map<String, serde_json::Value>>,
    /// Requested formats; ordinary defaults EOT/WOFF/WOFF2, variants WOFF/WOFF2.
    pub types: Option<Vec<FontType>>,
    /// Variant CSS modifier prefix (default `icon--`).
    pub variant_class_prefix: Option<String>,
}

#[derive(Clone)]
pub(super) struct ResolvedSourceOptions {
    pub common: ResolvedGenerationOptions,
    pub variants: Option<ResolvedVariants>,
    pub sources: Vec<Vec<LoadedSvgSource>>,
    pub css_template: Option<Arc<str>>,
    pub html_template: Option<Arc<str>>,
}

pub(super) fn resolve(
    options: GenerateWebfontsFromSourcesOptions,
) -> std::io::Result<ResolvedSourceOptions> {
    let settings = GenerationOptions {
        color_glyphs: options.color_glyphs,
        ascent: options.ascent,
        center_horizontally: options.center_horizontally,
        center_vertically: options.center_vertically,
        css: options.css,
        codepoints: options.codepoints,
        css_fonts_url: options.css_fonts_url,
        descent: options.descent,
        fixed_width: options.fixed_width,
        format_options: options.format_options,
        html: options.html,
        incremental: options.incremental,
        font_height: options.font_height,
        font_name: options.font_name,
        font_style: options.font_style,
        font_weight: options.font_weight,
        ligature: options.ligature,
        missing_glyphs: options.missing_glyphs,
        normalize: options.normalize,
        order: options.order,
        optimize_output: options.optimize_output,
        preserve_aspect_ratio: options.preserve_aspect_ratio,
        round: options.round,
        start_codepoint: options.start_codepoint,
        template_options: options.template_options,
        types: options.types,
        variant_class_prefix: options.variant_class_prefix,
    };
    let (variants, designs) = if let Some(variants) = options.variants {
        if !options.sources.is_empty() {
            return Err(invalid(
                "options.sources must be empty when variants are supplied",
            ));
        }
        let metadata = variants
            .iter()
            .map(|variant| VariantMetadata {
                name: &variant.name,
                weight: variant.weight,
                default: variant.default,
                source_count: variant.sources.len(),
            })
            .collect::<Vec<_>>();
        validate_variants(&settings, &metadata, "sources")?;
        validate_format_options(settings.format_options.as_ref())?;
        let resolved = resolve_variant_metadata(
            &metadata,
            variants
                .iter()
                .map(|variant| {
                    variant
                        .sources
                        .iter()
                        .map(|source| source.id.clone())
                        .collect()
                })
                .collect(),
            settings.variant_class_prefix.as_deref().unwrap_or("icon--"),
        )?;
        (
            Some(resolved),
            variants
                .into_iter()
                .map(|variant| variant.sources)
                .collect(),
        )
    } else {
        if options.sources.is_empty() {
            return Err(invalid("options.sources is empty"));
        }
        settings.validate_without_variants()?;
        validate_format_options(settings.format_options.as_ref())?;
        (None, vec![options.sources])
    };
    let mut common = settings.resolve(variants.is_some())?;
    common.filesystem_free = true;
    for name in common.explicit_codepoints.keys() {
        validate_name(name)?;
    }
    let sources = normalize_sources(designs)?;
    Ok(ResolvedSourceOptions {
        common,
        variants,
        sources,
        css_template: options.css_template.map(Arc::from),
        html_template: options.html_template.map(Arc::from),
    })
}

fn normalize_sources(designs: Vec<Vec<SvgSource>>) -> std::io::Result<Vec<Vec<LoadedSvgSource>>> {
    let mut definitions = HashMap::<String, LoadedSvgSource>::new();
    designs
        .into_iter()
        .map(|sources| {
            let mut ids = HashSet::new();
            let sources = sources
                .into_iter()
                .map(|source| {
                    if source.id.is_empty() || !ids.insert(source.id.clone()) {
                        return Err(invalid(format!(
                            "Source ID {:?} must be nonempty and unique within a design",
                            source.id
                        )));
                    }
                    validate_name(&source.name)?;
                    if let Some(existing) = definitions.get(&source.id) {
                        if existing.glyph_name != source.name
                            || existing.contents.as_ref() != source.svg
                        {
                            return Err(invalid(format!(
                                "Conflicting definitions for source ID {:?}",
                                source.id
                            )));
                        }
                        return Ok(existing.clone());
                    }
                    let loaded = LoadedSvgSource {
                        id: source.id,
                        glyph_name: source.name,
                        contents: source.svg.into(),
                    };
                    definitions.insert(loaded.id.clone(), loaded.clone());
                    Ok(loaded)
                })
                .collect::<std::io::Result<Vec<_>>>()?;
            validate_glyph_names(&sources)?;
            Ok(sources)
        })
        .collect()
}

pub(super) fn invalid(message: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Error {
    Error::new(ErrorKind::InvalidInput, message)
}

pub(super) fn validate_name(name: &str) -> std::io::Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(invalid(format!(
            "Glyph name {name:?} must contain only Unicode letters/numbers, hyphens or underscores"
        )));
    }
    Ok(())
}
