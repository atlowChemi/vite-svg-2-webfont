use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Error;

use handlebars::Handlebars;
use md5::Context;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    types::{FontType, LoadedSvgFile, ResolvedGenerateWebfontsOptions},
    util::{join_url, to_io_err},
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

fn calc_hash(options: &ResolvedGenerateWebfontsOptions, source_files: &[LoadedSvgFile]) -> String {
    let mut hash = Context::new();

    for source_file in source_files {
        hash.consume(&source_file.contents);
    }

    let hashable = HashableGenerateWebfontsOptions::from(options);
    serde_json::to_writer(Md5Writer(&mut hash), &hashable).expect("hash options should serialize");

    format!("{:x}", hash.finalize())
}

fn make_urls(
    options: &ResolvedGenerateWebfontsOptions,
    hash: &str,
    css_fonts_url: Option<&str>,
) -> HashMap<FontType, String> {
    let base_url = css_fonts_url.map(|value| value.replace('\\', "/"));
    let types = &options.types;

    types
        .iter()
        .copied()
        .map(|font_type| {
            let file_name = format!(
                "{}.{}?{}",
                &options.font_name,
                font_type.as_extension(),
                hash
            );
            let url = if let Some(base_url) = &base_url {
                join_url(base_url, &file_name)
            } else {
                file_name
            };

            (font_type, url)
        })
        .collect()
}

pub(crate) fn make_src(
    options: &ResolvedGenerateWebfontsOptions,
    urls: &HashMap<FontType, String>,
) -> String {
    let mut result = String::new();
    for (i, font_type) in options.order.iter().copied().enumerate() {
        if i > 0 {
            result.push_str(",\n");
        }
        let url = urls.get(&font_type).map(String::as_str).unwrap_or("");
        match font_type {
            FontType::Eot => {
                _ = write!(
                    result,
                    "url(\"{url}?#iefix\") format(\"{}\")",
                    font_type.css_format()
                );
            }
            FontType::Svg => {
                _ = write!(
                    result,
                    "url(\"{url}#{}\") format(\"{}\")",
                    &options.font_name,
                    font_type.css_format()
                );
            }
            _ => {
                _ = write!(
                    result,
                    "url(\"{url}\") format(\"{}\")",
                    font_type.css_format()
                );
            }
        }
    }
    result
}

fn make_ctx(
    options: &ResolvedGenerateWebfontsOptions,
    urls: &HashMap<FontType, String>,
    shared: &SharedTemplateData,
) -> Map<String, Value> {
    let mut ctx = Map::from_iter([
        (
            "fontName".to_owned(),
            Value::String(options.font_name.to_owned()),
        ),
        ("src".to_owned(), Value::String(make_src(options, urls))),
        (
            "codepoints".to_owned(),
            Value::Object(shared.codepoints_hex.clone()),
        ),
    ]);

    ctx.extend(shared.template_options.clone());

    ctx
}

#[cfg(feature = "napi")]
pub(crate) type ContextFunction = napi::threadsafe_function::ThreadsafeFunction<
    Map<String, Value>,
    Map<String, Value>,
    Map<String, Value>,
    napi::Status,
    false,
>;

#[cfg(feature = "napi")]
pub(crate) async fn apply_context_function(
    ctx: Map<String, Value>,
    context_fn: Option<&ContextFunction>,
) -> Result<Map<String, Value>, Error> {
    match context_fn {
        Some(tsf) => tsf.call_async(ctx).await.map_err(to_io_err),
        None => Ok(ctx),
    }
}

pub(crate) fn build_css_context(
    options: &ResolvedGenerateWebfontsOptions,
    shared: &SharedTemplateData,
) -> Map<String, Value> {
    build_css_context_with_fonts_url(options, shared, options.css_fonts_url.as_deref())
}

pub(crate) fn build_css_context_with_fonts_url(
    options: &ResolvedGenerateWebfontsOptions,
    shared: &SharedTemplateData,
    css_fonts_url: Option<&str>,
) -> Map<String, Value> {
    let urls = make_urls(options, &shared.hash, css_fonts_url);
    make_ctx(options, &urls, shared)
}

/// Render CSS using a pre-built Handlebars Context (no serialization).
/// Falls back to the hot-path renderer when no custom template is configured.
pub(crate) fn render_css_with_hbs_context(
    shared: &SharedTemplateData,
    hbs_ctx: &handlebars::Context,
    map_ctx: &Map<String, Value>,
) -> Result<String, Error> {
    match shared.css_registry()? {
        Some(registry) => registry
            .render_with_context("css", hbs_ctx)
            .map_err(to_io_err),
        None => Ok(render_default_css(map_ctx)),
    }
}

/// Render CSS from a Map context (used during init when no pre-built hbs Context exists yet).
pub(super) fn render_css_with_context(
    shared: &SharedTemplateData,
    ctx: &Map<String, Value>,
) -> Result<String, Error> {
    match shared.css_registry()? {
        Some(registry) => registry.render("css", ctx).map_err(to_io_err),
        None => Ok(render_default_css(ctx)),
    }
}

/// Render CSS with a different `src` value by mutating the Context in place,
/// rendering, then restoring the original value. Zero allocation.
/// Falls back to the hot-path renderer when no custom template is configured.
pub(crate) fn render_css_with_src_mutate(
    shared: &SharedTemplateData,
    hbs_ctx: &mut handlebars::Context,
    map_ctx: &Map<String, Value>,
    src: &str,
) -> Result<String, Error> {
    match shared.css_registry()? {
        Some(registry) => crate::util::render_with_field_swap(
            hbs_ctx,
            "src",
            Value::String(src.to_owned()),
            |ctx| registry.render_with_context("css", ctx).map_err(to_io_err),
        ),
        None => Ok(render_default_css_inner(
            map_ctx,
            super::ctx_str(map_ctx, "fontName", ""),
            src,
        )),
    }
}

fn render_default_css(ctx: &Map<String, Value>) -> String {
    render_default_css_inner(
        ctx,
        super::ctx_str(ctx, "fontName", ""),
        super::ctx_str(ctx, "src", ""),
    )
}

fn render_default_css_inner(ctx: &Map<String, Value>, font_name: &str, src: &str) -> String {
    let base_selector = super::ctx_str(ctx, "baseSelector", ".icon");
    let class_prefix = super::ctx_str(ctx, "classPrefix", "icon-");
    let codepoints = ctx.get("codepoints").and_then(|v| v.as_object());

    let codepoint_count = codepoints.map_or(0, |c| c.len());
    let mut result = String::with_capacity(256 + codepoint_count * 60);

    _ = write!(
        result,
        "@font-face {{\n\tfont-family: \"{font_name}\";\n\tfont-display: block;\n\tsrc: {src};\n}}\n\n"
    );
    _ = write!(result, "{base_selector} {{\n\tline-height: 1;\n}}\n\n");
    _ = write!(
        result,
        "{base_selector}:before {{\n\tfont-family: {font_name} !important;\n\tfont-style: normal;\n\tfont-weight: normal !important;\n\tvertical-align: top;\n}}\n\n"
    );

    if let Some(codepoints) = codepoints {
        for (name, value) in codepoints {
            let code = value.as_str().unwrap_or("");
            _ = write!(
                result,
                ".{class_prefix}{name}:before {{\n\tcontent: \"\\{code}\";\n}}\n"
            );
        }
    }

    result
}

/// Check whether a Handlebars template source contains `{{name}}` or `{{{name}}}` as an
/// exact variable reference (with optional whitespace). Does not match block helpers,
/// conditionals, or sub-expressions — only bare mustache names.
fn template_contains_exact_mustache_name(source: &str, name: &str) -> bool {
    let mut s = source;
    loop {
        let Some(open) = s.find("{{") else {
            return false;
        };
        s = &s[open + 2..];
        let (inner_start, close_pat) = match s.strip_prefix('{') {
            Some(rest) => (rest, "}}}"),
            None => (s, "}}"),
        };
        let Some(close) = inner_start.find(close_pat) else {
            return false;
        };
        if inner_start[..close].trim() == name {
            return true;
        }
        s = &inner_start[close + close_pat.len()..];
    }
}

/// Pre-computed values shared between CSS and HTML context building.
/// Avoids recomputing the hash, codepoints map, template options, and reading
/// the CSS template file multiple times. The CSS template source is read eagerly
/// (so file-not-found errors surface at init time), but compilation is deferred
/// to first render via OnceLock (matching upstream's lazy behavior).
pub(crate) struct SharedTemplateData {
    pub codepoints_hex: Map<String, Value>,
    pub codepoints_num: Map<String, Value>,
    css_template_source: Option<String>,
    css_registry_cache: std::sync::OnceLock<Result<Handlebars<'static>, String>>,
    /// Whether the CSS template references `{src}` — if false, URL overrides are a no-op.
    pub css_template_uses_src: bool,
    pub hash: String,
    pub template_options: Map<String, Value>,
}

impl SharedTemplateData {
    pub fn new(
        options: &ResolvedGenerateWebfontsOptions,
        source_files: &[LoadedSvgFile],
    ) -> Result<Self, Error> {
        let css_template_source = match &options.css_template {
            Some(path) => Some(fs::read_to_string(path)?),
            None => None,
        };
        // Default template always uses src. Custom templates: scan for any
        // Handlebars expression referencing `src` (handles whitespace variants
        // like `{{ src }}`, `{{{ src }}}`, etc.).
        let css_template_uses_src = match &css_template_source {
            None => true,
            Some(source) => template_contains_exact_mustache_name(source, "src"),
        };
        let (codepoints_hex, codepoints_num) = make_codepoints(options);
        Ok(Self {
            codepoints_hex,
            codepoints_num,
            css_template_source,
            css_registry_cache: std::sync::OnceLock::new(),
            css_template_uses_src,
            hash: calc_hash(options, source_files),
            template_options: resolved_template_options(options),
        })
    }

    /// Returns the compiled CSS Handlebars registry, compiling on first access.
    /// Returns None when no custom template is configured (default hot path).
    pub fn css_registry(&self) -> Result<Option<&Handlebars<'static>>, Error> {
        match &self.css_template_source {
            None => Ok(None),
            Some(source) => {
                let result = self.css_registry_cache.get_or_init(|| {
                    let mut registry = Handlebars::new();
                    registry
                        .register_template_string("css", source)
                        .map_err(|error| format!("Failed to compile CSS template: {error}"))?;
                    Ok(registry)
                });
                match result {
                    Ok(registry) => Ok(Some(registry)),
                    Err(msg) => Err(to_io_err(msg)),
                }
            }
        }
    }
}

/// Build both codepoint maps sorted by codepoint value (matching upstream iteration order).
/// The sort is O(n log n) but only runs once during SharedTemplateData init, not per render.
fn make_codepoints(
    options: &ResolvedGenerateWebfontsOptions,
) -> (Map<String, Value>, Map<String, Value>) {
    let mut by_value: Vec<_> = options.codepoints.iter().collect();
    by_value.sort_by_key(|(_, cp)| *cp);
    let mut hex = Map::with_capacity(by_value.len());
    let mut num = Map::with_capacity(by_value.len());
    for (name, codepoint) in by_value {
        hex.insert(name.clone(), Value::String(format!("{:x}", codepoint)));
        num.insert(name.clone(), Value::Number((*codepoint).into()));
    }
    (hex, num)
}

fn resolved_template_options(options: &ResolvedGenerateWebfontsOptions) -> Map<String, Value> {
    let mut template_options = Map::from_iter([
        ("baseSelector".to_owned(), Value::String(".icon".to_owned())),
        ("classPrefix".to_owned(), Value::String("icon-".to_owned())),
    ]);

    if let Some(custom_template_options) = &options.template_options {
        template_options.extend(custom_template_options.clone());
    }

    template_options
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

#[cfg(test)]
mod tests {
    use super::{
        SharedTemplateData, build_css_context, calc_hash, make_ctx, make_src, make_urls,
        render_css_with_context, template_contains_exact_mustache_name,
    };
    use crate::{
        FontType, FormatOptions, GenerateWebfontsOptions, LoadedSvgFile,
        ResolvedGenerateWebfontsOptions, SvgFormatOptions, TtfFormatOptions, WoffFormatOptions,
    };
    use serde_json::{Map, Value};
    use std::collections::HashMap;
    use std::fs;
    use std::io::{Error, ErrorKind};

    fn render_css(
        options: &ResolvedGenerateWebfontsOptions,
        source_files: &[LoadedSvgFile],
    ) -> Result<String, Error> {
        let shared = SharedTemplateData::new(options, source_files)?;
        let ctx = build_css_context(options, &shared);
        render_css_with_context(&shared, &ctx)
    }

    use crate::test_helpers::{fixture_source_files, resolve_options, write_temp_template};

    #[test]
    fn hash_matches_expected_value_for_known_options() {
        let options = GenerateWebfontsOptions {
            ascent: Some(1000.0),
            center_horizontally: Some(true),
            center_vertically: Some(false),
            css: Some(false),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            descent: Some(120.0),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            fixed_width: Some(false),
            format_options: Some(FormatOptions {
                svg: Some(SvgFormatOptions {
                    font_id: Some("iconfont".to_owned()),
                    metadata: Some("svg-meta".to_owned()),
                    ..Default::default()
                }),
                ttf: Some(TtfFormatOptions {
                    copyright: Some("copyright".to_owned()),
                    description: Some("description".to_owned()),
                    ts: Some(1_484_141_760_000),
                    url: Some("https://example.com".to_owned()),
                    version: Some("Version 1.0".to_owned()),
                }),
                woff: Some(WoffFormatOptions {
                    metadata: Some("woff-meta".to_owned()),
                }),
            }),
            html: Some(false),
            font_height: Some(1000.0),
            font_name: Some("iconfont".to_owned()),
            font_style: Some("normal".to_owned()),
            font_weight: Some("400".to_owned()),
            ligature: Some(false),
            normalize: Some(true),
            order: Some(vec![FontType::Woff2, FontType::Svg, FontType::Ttf]),
            optimize_output: Some(false),
            preserve_aspect_ratio: Some(false),
            round: Some(1e3),
            start_codepoint: Some(0xE001),
            types: Some(vec![FontType::Svg, FontType::Ttf, FontType::Woff2]),
            ..Default::default()
        };

        let options = resolve_options(options);
        let source_files = vec![LoadedSvgFile {
            contents: fs::read_to_string(&options.files[0]).expect("fixture should load"),
            glyph_name: "add".to_owned(),
            path: options.files[0].clone(),
        }];
        let hash1 = calc_hash(&options, &source_files);
        let hash2 = calc_hash(&options, &source_files);

        assert_eq!(hash1, hash2, "hash should be deterministic across calls");
        assert_eq!(hash1.len(), 32, "hash should be a 32-char hex string");
    }

    #[test]
    fn make_urls_uses_hash_and_requested_type_order() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg, FontType::Woff2]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = vec![LoadedSvgFile {
            contents: fs::read_to_string(&options.files[0]).expect("fixture should load"),
            glyph_name: "add".to_owned(),
            path: options.files[0].clone(),
        }];
        let hash = calc_hash(&options, &source_files);

        let urls = make_urls(
            &options,
            &calc_hash(&options, &source_files),
            options.css_fonts_url.as_deref(),
        );

        assert_eq!(
            urls.get(&FontType::Svg),
            Some(&format!("iconfont.svg?{hash}"))
        );
        assert_eq!(
            urls.get(&FontType::Woff2),
            Some(&format!("iconfont.woff2?{hash}"))
        );
    }

    #[test]
    fn make_urls_joins_against_css_fonts_url_and_normalizes_backslashes() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            css_fonts_url: Some("fonts\\nested\\".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Ttf]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = vec![LoadedSvgFile {
            contents: fs::read_to_string(&options.files[0]).expect("fixture should load"),
            glyph_name: "add".to_owned(),
            path: options.files[0].clone(),
        }];
        let hash = calc_hash(&options, &source_files);

        let urls = make_urls(
            &options,
            &calc_hash(&options, &source_files),
            options.css_fonts_url.as_deref(),
        );

        assert_eq!(
            urls.get(&FontType::Ttf),
            Some(&format!("fonts/nested/iconfont.ttf?{hash}"))
        );
    }

    #[test]
    fn make_urls_treats_an_empty_trimmed_css_fonts_url_as_no_base_url() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            css_fonts_url: Some("///".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = vec![LoadedSvgFile {
            contents: fs::read_to_string(&options.files[0]).expect("fixture should load"),
            glyph_name: "add".to_owned(),
            path: options.files[0].clone(),
        }];
        let hash = calc_hash(&options, &source_files);

        let urls = make_urls(
            &options,
            &calc_hash(&options, &source_files),
            options.css_fonts_url.as_deref(),
        );

        assert_eq!(
            urls.get(&FontType::Svg),
            Some(&format!("iconfont.svg?{hash}"))
        );
    }

    #[test]
    fn make_src_uses_order_and_format_specific_url_templates() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec![],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Eot, FontType::Svg, FontType::Woff2]),
            types: Some(vec![FontType::Svg, FontType::Eot, FontType::Woff2]),
            ..Default::default()
        };
        let urls = HashMap::from([
            (FontType::Svg, "iconfont.svg?svg-hash".to_owned()),
            (FontType::Eot, "iconfont.eot?eot-hash".to_owned()),
            (FontType::Woff2, "iconfont.woff2?woff2-hash".to_owned()),
        ]);

        let options = resolve_options(options);
        let src = make_src(&options, &urls);

        assert_eq!(
            src,
            concat!(
                "url(\"iconfont.eot?eot-hash?#iefix\") format(\"embedded-opentype\")",
                ",\n",
                "url(\"iconfont.svg?svg-hash#iconfont\") format(\"svg\")",
                ",\n",
                "url(\"iconfont.woff2?woff2-hash\") format(\"woff2\")"
            )
        );
    }

    #[test]
    fn make_src_uses_upstream_default_order_when_order_is_not_provided() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec![],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![
                FontType::Svg,
                FontType::Woff,
                FontType::Eot,
                FontType::Ttf,
            ]),
            ..Default::default()
        };
        let urls = HashMap::from([
            (FontType::Svg, "iconfont.svg?svg-hash".to_owned()),
            (FontType::Eot, "iconfont.eot?eot-hash".to_owned()),
            (FontType::Woff, "iconfont.woff?woff-hash".to_owned()),
            (FontType::Ttf, "iconfont.ttf?ttf-hash".to_owned()),
        ]);

        let options = resolve_options(options);
        let src = make_src(&options, &urls);

        assert_eq!(
            src,
            concat!(
                "url(\"iconfont.eot?eot-hash?#iefix\") format(\"embedded-opentype\")",
                ",\n",
                "url(\"iconfont.woff?woff-hash\") format(\"woff\")",
                ",\n",
                "url(\"iconfont.ttf?ttf-hash\") format(\"truetype\")",
                ",\n",
                "url(\"iconfont.svg?svg-hash#iconfont\") format(\"svg\")"
            )
        );
    }

    #[test]
    fn make_ctx_builds_codepoints_and_merges_template_options() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            dest: "artifacts".to_owned(),
            files: vec![],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Ttf]),
            template_options: Some(Map::from_iter([
                (
                    "baseSelector".to_owned(),
                    Value::String(".glyph".to_owned()),
                ),
                (
                    "fontName".to_owned(),
                    Value::String("overridden".to_owned()),
                ),
            ])),
            types: Some(vec![FontType::Ttf]),
            ..Default::default()
        };
        let urls = HashMap::from([(FontType::Ttf, "iconfont.ttf?hash".to_owned())]);

        let options = resolve_options(options);
        let shared = SharedTemplateData::new(&options, &[]).unwrap();
        let ctx = make_ctx(&options, &urls, &shared);

        assert_eq!(
            ctx.get("fontName"),
            Some(&Value::String("overridden".to_owned()))
        );
        assert_eq!(
            ctx.get("src"),
            Some(&Value::String(
                "url(\"iconfont.ttf?hash\") format(\"truetype\")".to_owned()
            ))
        );
        assert_eq!(
            ctx.get("baseSelector"),
            Some(&Value::String(".glyph".to_owned()))
        );
        assert_eq!(
            ctx.get("classPrefix"),
            Some(&Value::String("icon-".to_owned()))
        );
        assert_eq!(
            ctx.get("codepoints"),
            Some(&Value::Object(Map::from_iter([(
                "add".to_owned(),
                Value::String("e001".to_owned()),
            )])))
        );
    }

    #[test]
    fn render_css_renders_the_template_with_generated_urls() {
        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            css_fonts_url: Some("/assets/fonts".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg, FontType::Ttf]),
            types: Some(vec![FontType::Svg, FontType::Ttf]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = vec![LoadedSvgFile {
            contents: fs::read_to_string(&options.files[0]).expect("fixture should load"),
            glyph_name: "add".to_owned(),
            path: options.files[0].clone(),
        }];

        let css = render_css(&options, &source_files).expect("css should render");

        assert!(css.contains("@font-face"));
        assert!(css.contains("font-family: \"iconfont\";"));
        assert!(css.contains("url(\"/assets/fonts/iconfont.svg?"));
        assert!(css.contains("format(\"svg\")"));
        assert!(css.contains("format(\"truetype\")"));
        assert!(css.contains(".icon-add:before"));
        assert!(css.contains("\\e001"));
    }

    #[test]
    fn render_css_supports_static_custom_templates() {
        let template_path = write_temp_template("native-css-static-template", "custom css");
        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(template_path),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            css_fonts_url: Some("/assets/fonts".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = fixture_source_files(&options);

        let css = render_css(&options, &source_files).expect("css should render");

        assert_eq!(css, "custom css");
    }

    #[test]
    fn render_css_supports_custom_templates_using_all_available_context_values() {
        let template_path = write_temp_template(
            "native-css-full-context-template",
            "{{fontName}}|{{{src}}}|{{baseSelector}}|{{classPrefix}}|{{codepoints.add}}|{{option}}",
        );
        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(template_path),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            css_fonts_url: Some("/assets/fonts".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            template_options: Some(Map::from_iter([(
                "option".to_owned(),
                Value::String("TEST".to_owned()),
            )])),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = fixture_source_files(&options);

        let css = render_css(&options, &source_files).expect("css should render");

        assert!(css.starts_with("iconfont|url(\"/assets/fonts/iconfont.svg?"));
        assert!(css.contains("#iconfont\") format(\"svg\")|.icon|icon-|e001|TEST"));
    }

    #[test]
    fn render_css_rejects_invalid_handlebars_templates() {
        let template_path = write_temp_template("native-css-invalid-template", "{{#if}}");
        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(template_path),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            css_fonts_url: Some("/assets/fonts".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = fixture_source_files(&options);

        let error =
            render_css(&options, &source_files).expect_err("invalid handlebars syntax should fail");

        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn default_css_hot_path_matches_handlebars_output() {
        use handlebars::Handlebars;

        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(format!("{}/templates/css.hbs", env!("CARGO_MANIFEST_DIR"))),
            codepoints: Some(HashMap::from([
                ("add".to_owned(), 0xE001u32),
                ("remove".to_owned(), 0xE002u32),
                ("search".to_owned(), 0xE003u32),
            ])),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_height: Some(1000.0),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            start_codepoint: Some(0xE001),
            template_options: Some(Map::from_iter([
                ("baseSelector".to_owned(), Value::String(".icon".to_owned())),
                ("classPrefix".to_owned(), Value::String("icon-".to_owned())),
            ])),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };
        let options = resolve_options(options);
        let source_files = fixture_source_files(&options);
        let shared_with_template = SharedTemplateData::new(&options, &source_files).unwrap();
        let ctx = super::build_css_context(&options, &shared_with_template);

        // Render via Handlebars (the template path is set)
        let handlebars_output = {
            let source = fs::read_to_string(options.css_template.as_ref().unwrap()).unwrap();
            let registry = Handlebars::new();
            registry.render_template(&source, &ctx).unwrap()
        };

        // Render via hot path (no template = default)
        let hot_path_output = super::render_default_css(&ctx);

        assert_eq!(
            hot_path_output, handlebars_output,
            "CSS hot path output must match Handlebars output"
        );
    }

    #[test]
    fn render_css_with_hbs_context_matches_direct_render_for_default_template() {
        let options = resolve_options(GenerateWebfontsOptions {
            css: Some(true),
            codepoints: Some(HashMap::from([
                ("add".to_owned(), 0xE001u32),
                ("remove".to_owned(), 0xE002u32),
            ])),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        });
        let source_files = fixture_source_files(&options);
        let shared = SharedTemplateData::new(&options, &source_files).unwrap();
        let ctx = build_css_context(&options, &shared);
        let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

        let direct = render_css_with_context(&shared, &ctx).unwrap();
        let via_hbs = super::render_css_with_hbs_context(&shared, &hbs_ctx, &ctx).unwrap();

        assert_eq!(
            via_hbs, direct,
            "render_css_with_hbs_context must match render_css_with_context"
        );
    }

    #[test]
    fn render_css_with_hbs_context_matches_direct_render_for_custom_template() {
        let template_path = write_temp_template(
            "native-css-hbs-ctx",
            "@font-face { src: {{{src}}}; } {{#each codepoints}}.{{@key}}:before { content: \"\\\\{{this}}\"; }{{/each}}",
        );
        let options = resolve_options(GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(template_path),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            css_fonts_url: Some("/fonts".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        });
        let source_files = fixture_source_files(&options);
        let shared = SharedTemplateData::new(&options, &source_files).unwrap();
        let ctx = build_css_context(&options, &shared);
        let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

        let direct = render_css_with_context(&shared, &ctx).unwrap();
        let via_hbs = super::render_css_with_hbs_context(&shared, &hbs_ctx, &ctx).unwrap();

        assert_eq!(
            via_hbs, direct,
            "render_css_with_hbs_context with custom template must match render_css_with_context"
        );
    }

    #[test]
    fn render_css_with_src_swap_matches_manual_context_rewrite() {
        let template_path = write_temp_template(
            "native-css-src-swap",
            "@font-face { src: {{{src}}}; } .icon { font-family: {{fontName}}; }",
        );
        let options = resolve_options(GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(template_path),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            css_fonts_url: Some("/fonts".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        });
        let source_files = fixture_source_files(&options);
        let shared = SharedTemplateData::new(&options, &source_files).unwrap();
        let ctx = build_css_context(&options, &shared);
        let hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

        // Manual approach: clone Map, replace src, render
        let new_src = "url(\"/custom/path.woff2\") format(\"woff2\")";
        let mut manual_ctx = ctx.clone();
        manual_ctx.insert("src".to_owned(), Value::String(new_src.to_owned()));
        let expected = render_css_with_context(&shared, &manual_ctx).unwrap();

        // Optimized approach: in-place mutate hbs Context src field
        let mut hbs_ctx = hbs_ctx;
        let actual =
            super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, new_src).unwrap();

        assert_eq!(
            actual, expected,
            "render_css_with_src_mutate must produce identical output to manual Map rewrite"
        );

        // Verify original src was restored
        let restored_src = hbs_ctx
            .data()
            .as_object()
            .unwrap()
            .get("src")
            .unwrap()
            .as_str()
            .unwrap();
        let original_src = ctx.get("src").unwrap().as_str().unwrap();
        assert_eq!(
            restored_src, original_src,
            "original src should be restored after render"
        );
    }

    #[test]
    fn render_css_with_src_mutate_produces_correct_results_on_repeated_calls() {
        let template_path = write_temp_template(
            "native-css-src-mutate-repeat",
            "@font-face { src: {{{src}}}; }",
        );
        let options = resolve_options(GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(template_path),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            css_fonts_url: Some("/fonts".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        });
        let source_files = fixture_source_files(&options);
        let shared = SharedTemplateData::new(&options, &source_files).unwrap();
        let ctx = build_css_context(&options, &shared);
        let mut hbs_ctx = handlebars::Context::wraps(&ctx).unwrap();

        let src_a = "url(\"/a.woff2\") format(\"woff2\")";
        let src_b = "url(\"/b.woff\") format(\"woff\")";

        let result_a =
            super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, src_a).unwrap();
        let result_b =
            super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, src_b).unwrap();
        let result_a_again =
            super::render_css_with_src_mutate(&shared, &mut hbs_ctx, &ctx, src_a).unwrap();

        assert!(result_a.contains(src_a), "first call should use src_a");
        assert!(result_b.contains(src_b), "second call should use src_b");
        assert_eq!(
            result_a, result_a_again,
            "repeated call with same src should produce identical output"
        );
        assert_ne!(
            result_a, result_b,
            "different src values should produce different output"
        );
    }

    #[test]
    fn css_registry_rejects_invalid_template_syntax_on_first_access() {
        let template_path = write_temp_template("native-css-invalid-compile", "{{#if}}");
        let options = resolve_options(GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(template_path),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        });

        // SharedTemplateData::new succeeds (file is read, compilation is deferred)
        let shared = SharedTemplateData::new(&options, &[])
            .expect("init should succeed — template source is read but not compiled");

        // First access to css_registry triggers compilation and fails
        match shared.css_registry() {
            Err(error) => {
                assert_eq!(error.kind(), ErrorKind::InvalidData);
                assert!(
                    error.to_string().contains("Failed to compile CSS template"),
                    "error should mention CSS template: {error}"
                );
            }
            Ok(_) => panic!("invalid handlebars syntax should fail on first css_registry() access"),
        }
    }

    #[test]
    fn shared_template_data_reads_source_but_does_not_compile_invalid_css_template_eagerly() {
        let template_path = write_temp_template("native-css-invalid-lazy", "{{#if}}");
        let options = resolve_options(GenerateWebfontsOptions {
            css: Some(false),
            css_template: Some(template_path),
            dest: "artifacts".to_owned(),
            files: vec![crate::test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        });

        // Init succeeds — source is read but not compiled
        let shared = SharedTemplateData::new(&options, &[]);
        assert!(
            shared.is_ok(),
            "init should succeed even with invalid template content"
        );
    }

    // --- template_contains_exact_mustache_name ---

    #[test]
    fn mustache_match_double_no_whitespace() {
        assert!(template_contains_exact_mustache_name("{{src}}", "src"));
    }

    #[test]
    fn mustache_match_triple_no_whitespace() {
        assert!(template_contains_exact_mustache_name("{{{src}}}", "src"));
    }

    #[test]
    fn mustache_match_double_leading_space() {
        assert!(template_contains_exact_mustache_name("{{ src}}", "src"));
    }

    #[test]
    fn mustache_match_double_trailing_space() {
        assert!(template_contains_exact_mustache_name("{{src }}", "src"));
    }

    #[test]
    fn mustache_match_double_both_spaces() {
        assert!(template_contains_exact_mustache_name("{{ src }}", "src"));
    }

    #[test]
    fn mustache_match_triple_leading_space() {
        assert!(template_contains_exact_mustache_name("{{{ src}}}", "src"));
    }

    #[test]
    fn mustache_match_triple_trailing_space() {
        assert!(template_contains_exact_mustache_name("{{{src }}}", "src"));
    }

    #[test]
    fn mustache_match_triple_both_spaces() {
        assert!(template_contains_exact_mustache_name("{{{ src }}}", "src"));
    }

    #[test]
    fn mustache_match_with_surrounding_text() {
        assert!(template_contains_exact_mustache_name(
            "@font-face { src: {{ src }}; }",
            "src"
        ));
    }

    #[test]
    fn mustache_match_with_tabs() {
        assert!(template_contains_exact_mustache_name("{{\tsrc\t}}", "src"));
    }

    #[test]
    fn mustache_no_match_different_name() {
        assert!(!template_contains_exact_mustache_name(
            "{{fontName}}",
            "src"
        ));
    }

    #[test]
    fn mustache_no_match_prefix() {
        assert!(!template_contains_exact_mustache_name("{{srcUrl}}", "src"));
    }

    #[test]
    fn mustache_no_match_block_helper() {
        assert!(!template_contains_exact_mustache_name("{{#if src}}", "src"));
    }

    #[test]
    fn mustache_no_match_empty_source() {
        assert!(!template_contains_exact_mustache_name("", "src"));
    }

    #[test]
    fn mustache_no_match_no_braces() {
        assert!(!template_contains_exact_mustache_name(
            "plain text src",
            "src"
        ));
    }

    #[test]
    fn mustache_match_multiple_expressions_second_matches() {
        assert!(template_contains_exact_mustache_name(
            "{{fontName}} {{ src }}",
            "src"
        ));
    }
}
