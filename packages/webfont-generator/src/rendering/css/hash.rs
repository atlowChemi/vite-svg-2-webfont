use md5::Context;
use serde::Serialize;
use serde_json::{Map, Value};

use super::context::resolved_template_options;
use crate::{
    input::{LoadedSvgFile, ResolvedGenerateWebfontsOptions},
    types::FontType,
};

/// Wraps md5::Context as an io::Write so serde_json can stream directly into
/// the hash without allocating an intermediate String.
struct Md5Writer<'a>(&'a mut Context);

impl std::io::Write for Md5Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.consume(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn calc_hash(
    options: &ResolvedGenerateWebfontsOptions,
    source_files: &[LoadedSvgFile],
) -> String {
    let mut hash = Context::new();

    for source_file in source_files {
        hash.consume(source_file.contents.as_bytes());
    }

    let hashable = HashableGenerateWebfontsOptions::from(options);
    serde_json::to_writer(Md5Writer(&mut hash), &hashable).expect("hash options should serialize");

    format!("{:x}", hash.finalize())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HashableGenerateWebfontsOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    ascent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center_horizontally: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center_vertically: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    css: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    css_template: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    codepoints: Option<Vec<HashableCodepointAssignment<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    css_fonts_url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    descent: Option<f64>,
    files: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_width: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format_options: Option<HashableFormatOptions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_template: Option<&'a str>,
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

impl<'a> From<&'a ResolvedGenerateWebfontsOptions> for HashableGenerateWebfontsOptions<'a> {
    fn from(options: &'a ResolvedGenerateWebfontsOptions) -> Self {
        Self {
            ascent: options.ascent,
            center_horizontally: options.center_horizontally,
            center_vertically: options.center_vertically,
            css: Some(options.css),
            css_template: options.css_template.as_deref(),
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
            files: &options.files,
            fixed_width: options.fixed_width,
            format_options: options
                .format_options
                .as_ref()
                .map(HashableFormatOptions::from),
            html: Some(options.html),
            html_template: options.html_template.as_deref(),
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
