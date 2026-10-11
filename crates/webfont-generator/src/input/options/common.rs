use std::collections::{BTreeMap, HashMap};

use crate::types::color::ResolvedColorSelection;
use crate::types::{
    ColorGlyphSelection, FontType, FormatOptions, MissingGlyphBehavior, MissingGlyphOptions,
};

use super::{
    resolve_font_type_order, resolved_font_types, validate_color_formats, validate_font_type_order,
};

/// Validation needs membership size, never the source paths or SVG payloads.
pub(super) struct VariantMetadata<'a> {
    pub name: &'a str,
    pub weight: Option<u16>,
    pub default: Option<bool>,
    pub source_count: usize,
}

/// Content-independent settings, retaining omission until validation and resolution.
#[derive(Default)]
pub(crate) struct GenerationOptions {
    pub color_glyphs: Option<ColorGlyphSelection>,
    pub ascent: Option<f64>,
    pub center_horizontally: Option<bool>,
    pub center_vertically: Option<bool>,
    pub css: Option<bool>,
    pub codepoints: Option<HashMap<String, u32>>,
    pub css_fonts_url: Option<String>,
    pub descent: Option<f64>,
    pub fixed_width: Option<bool>,
    pub format_options: Option<FormatOptions>,
    pub html: Option<bool>,
    pub incremental: Option<bool>,
    pub font_height: Option<f64>,
    pub font_name: Option<String>,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub ligature: Option<bool>,
    pub missing_glyphs: Option<MissingGlyphOptions>,
    pub normalize: Option<bool>,
    pub order: Option<Vec<FontType>>,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
    pub round: Option<f64>,
    pub start_codepoint: Option<u32>,
    pub template_options: Option<serde_json::Map<String, serde_json::Value>>,
    pub types: Option<Vec<FontType>>,
    pub variant_class_prefix: Option<String>,
}

/// Shared generation settings contain neither source paths nor template/output locations.
#[derive(Clone)]
pub(crate) struct ResolvedGenerationOptions {
    pub color_selection: Option<ResolvedColorSelection>,
    pub ascent: Option<f64>,
    pub center_horizontally: Option<bool>,
    pub center_vertically: Option<bool>,
    pub css: bool,
    /// Explicit and automatic assignments for the current glyph set.
    pub codepoints: BTreeMap<String, u32>,
    /// Stable caller assignments used when rebuilding codepoints after membership changes.
    pub explicit_codepoints: BTreeMap<String, u32>,
    pub css_fonts_url: Option<String>,
    pub descent: Option<f64>,
    pub fixed_width: Option<bool>,
    pub format_options: Option<FormatOptions>,
    pub html: bool,
    pub incremental: bool,
    pub font_height: Option<f64>,
    pub font_name: String,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub ligature: bool,
    pub missing_glyphs: MissingGlyphOptions,
    pub normalize: bool,
    pub order: Vec<FontType>,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
    pub round: Option<f64>,
    pub start_codepoint: u32,
    pub template_options: Option<serde_json::Map<String, serde_json::Value>>,
    pub types: Vec<FontType>,
}

impl GenerationOptions {
    pub(super) fn validate_without_variants(&self) -> std::io::Result<()> {
        if self.missing_glyphs.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "\"options.missingGlyphs\" requires \"options.variants\".",
            ));
        }
        if self.variant_class_prefix.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "\"options.variantClassPrefix\" requires \"options.variants\".",
            ));
        }
        Ok(())
    }

    pub(crate) fn resolve(self, variants: bool) -> std::io::Result<ResolvedGenerationOptions> {
        let types = resolved_font_types(self.types.as_deref(), variants);
        validate_font_type_order(self.order.as_deref(), &types)?;
        let order = resolve_font_type_order(self.order.as_deref(), &types);
        let color_selection = self.color_glyphs.map(ResolvedColorSelection::from);
        validate_color_formats(color_selection.as_ref(), &types)?;
        let explicit_codepoints: BTreeMap<_, _> =
            self.codepoints.unwrap_or_default().into_iter().collect();
        let svg_format = self
            .format_options
            .as_ref()
            .and_then(|format| format.svg.as_ref());
        let center_vertically = svg_format
            .and_then(|svg| svg.center_vertically)
            .or(self.center_vertically);
        let optimize_output = svg_format
            .and_then(|svg| svg.optimize_output)
            .or(self.optimize_output);
        let preserve_aspect_ratio = svg_format
            .and_then(|svg| svg.preserve_aspect_ratio)
            .or(self.preserve_aspect_ratio);

        Ok(ResolvedGenerationOptions {
            color_selection,
            ascent: self.ascent,
            center_horizontally: self.center_horizontally,
            center_vertically,
            css: self.css.unwrap_or(true),
            codepoints: explicit_codepoints.clone(),
            explicit_codepoints,
            css_fonts_url: self.css_fonts_url,
            descent: self.descent,
            fixed_width: self.fixed_width,
            format_options: self.format_options,
            html: self.html.unwrap_or(false),
            incremental: self.incremental.unwrap_or(false),
            font_height: self.font_height,
            font_name: self.font_name.unwrap_or_else(|| "iconfont".to_owned()),
            font_style: self.font_style,
            font_weight: self.font_weight,
            ligature: self.ligature.unwrap_or(true),
            missing_glyphs: self.missing_glyphs.unwrap_or(MissingGlyphOptions {
                behavior: MissingGlyphBehavior::Blank,
                variant: None,
            }),
            normalize: self.normalize.unwrap_or(true),
            order,
            optimize_output,
            preserve_aspect_ratio,
            round: self.round,
            start_codepoint: self.start_codepoint.unwrap_or(0xF101),
            template_options: self.template_options,
            types,
        })
    }
}
