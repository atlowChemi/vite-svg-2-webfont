//! The single list of generation settings that affect font output. Both the CSS font-URL hash
//! (`css::hash`) and the persistent cache keys (`cache`) are derived from it, so a setting that
//! changes font bytes must be added here to keep both correct.

use serde::Serialize;
use serde_json::{Map, Value};

use super::css::resolved_template_options;
#[cfg(feature = "fs")]
use crate::input::ResolvedFileGenerationOptions;
use crate::input::ResolvedGenerationOptions;
use crate::types::{FontType, color::ResolvedColorSelection};

/// Font-affecting settings: the URL-hash fingerprint without CSS/HTML-only fields, file paths,
/// and codepoints (callers that need codepoints hash them per glyph). Object keys are sorted.
pub(crate) fn font_settings_json(options: &ResolvedGenerationOptions) -> Value {
    let mut settings = FontFingerprint::from(options);
    settings.css = None;
    settings.css_template = None;
    settings.css_fonts_url = None;
    settings.html = None;
    settings.html_template = None;
    settings.order = None;
    settings.types = None;
    settings.codepoints = None;
    settings.files = None;
    settings.template_options = Map::new();
    let mut settings = serde_json::to_value(settings).expect("settings serialize");
    settings.sort_all_objects();
    settings
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FontFingerprint<'a> {
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    color_slots: &'a std::collections::BTreeMap<String, [u8; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color_glyphs: Option<&'a ResolvedColorSelection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ascent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center_horizontally: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center_vertically: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    css: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) css_template: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    codepoints: Option<Vec<HashableCodepointAssignment<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    css_fonts_url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    descent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_width: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format_options: Option<HashableFormatOptions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) html_template: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_style: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_weight: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ligature: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalize: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optimize_output: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preserve_aspect_ratio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    round: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_codepoint: Option<u32>,
    template_options: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    types: Option<Vec<&'static str>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashableCodepointAssignment<'a> {
    codepoint: u32,
    name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashableFormatOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    svg: Option<HashableSvgFormatOptions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttf: Option<HashableTtfFormatOptions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    woff: Option<HashableWoffFormatOptions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    woff2: Option<HashableWoff2FormatOptions>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashableWoff2FormatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    compression_quality: Option<u8>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashableSvgFormatOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    font_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashableTtfFormatOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    copyright: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashableWoffFormatOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<&'a str>,
}

#[cfg(feature = "fs")]
impl<'a> From<&'a ResolvedFileGenerationOptions> for FontFingerprint<'a> {
    fn from(options: &'a ResolvedFileGenerationOptions) -> Self {
        Self {
            files: Some(&options.files),
            css_template: options.css_template.as_deref(),
            html_template: options.html_template.as_deref(),
            ..Self::from(&options.common)
        }
    }
}

impl<'a> From<&'a ResolvedGenerationOptions> for FontFingerprint<'a> {
    fn from(options: &'a ResolvedGenerationOptions) -> Self {
        Self {
            color_slots: &options.color_slots,
            color_glyphs: options
                .color_selection
                .as_ref()
                .filter(|selection| match selection {
                    ResolvedColorSelection::All => true,
                    ResolvedColorSelection::Named(names) => !names.is_empty(),
                }),
            ascent: options.ascent,
            center_horizontally: options.center_horizontally,
            center_vertically: options.center_vertically,
            css: Some(options.css),
            css_template: None,
            codepoints: if options.codepoints.is_empty() {
                None
            } else {
                let mut codepoints = options
                    .codepoints
                    .iter()
                    .map(|(name, codepoint)| HashableCodepointAssignment {
                        codepoint: *codepoint,
                        name,
                    })
                    .collect::<Vec<_>>();
                codepoints.sort_by(|left, right| left.name.cmp(right.name));
                Some(codepoints)
            },
            css_fonts_url: options.css_fonts_url.as_deref(),
            descent: options.descent,
            files: None,
            fixed_width: options.fixed_width,
            format_options: options
                .format_options
                .as_ref()
                .map(HashableFormatOptions::from),
            html: Some(options.html),
            html_template: None,
            font_height: options.font_height,
            font_name: Some(&options.font_name),
            font_style: options.font_style.as_deref(),
            font_weight: options.font_weight.as_deref(),
            ligature: Some(options.ligature),
            normalize: Some(options.normalize),
            order: Some(
                options
                    .order
                    .iter()
                    .copied()
                    .map(FontType::as_extension)
                    .collect(),
            ),
            optimize_output: options.optimize_output,
            preserve_aspect_ratio: options.preserve_aspect_ratio,
            round: options.round,
            start_codepoint: Some(options.start_codepoint),
            template_options: resolved_template_options(options),
            types: Some({
                let types = &options.types;
                types.iter().copied().map(FontType::as_extension).collect()
            }),
        }
    }
}

impl<'a> From<&'a crate::types::FormatOptions> for HashableFormatOptions<'a> {
    fn from(options: &'a crate::types::FormatOptions) -> Self {
        Self {
            svg: options.svg.as_ref().map(HashableSvgFormatOptions::from),
            ttf: options.ttf.as_ref().map(HashableTtfFormatOptions::from),
            woff: options.woff.as_ref().map(HashableWoffFormatOptions::from),
            // Unlike other formats, an empty woff2 entry is omitted to keep pre-existing hashes stable.
            woff2: options
                .woff2
                .as_ref()
                .and_then(|woff2| woff2.compression_quality)
                .map(|quality| HashableWoff2FormatOptions {
                    compression_quality: Some(quality),
                }),
        }
    }
}

impl<'a> From<&'a crate::types::SvgFormatOptions> for HashableSvgFormatOptions<'a> {
    fn from(options: &'a crate::types::SvgFormatOptions) -> Self {
        Self {
            font_id: options.font_id.as_deref(),
            metadata: options.metadata.as_deref(),
        }
    }
}

impl<'a> From<&'a crate::types::TtfFormatOptions> for HashableTtfFormatOptions<'a> {
    fn from(options: &'a crate::types::TtfFormatOptions) -> Self {
        Self {
            copyright: options.copyright.as_deref(),
            description: options.description.as_deref(),
            ts: options.ts,
            url: options.url.as_deref(),
            version: options.version.as_deref(),
        }
    }
}

impl<'a> From<&'a crate::types::WoffFormatOptions> for HashableWoffFormatOptions<'a> {
    fn from(options: &'a crate::types::WoffFormatOptions) -> Self {
        Self {
            metadata: options.metadata.as_deref(),
        }
    }
}
