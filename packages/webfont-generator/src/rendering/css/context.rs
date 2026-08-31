use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Error;

use handlebars::Handlebars;
use serde_json::{Map, Value};

use super::dependencies::{TemplateDependencies, template_dependencies};
use super::hash::calc_hash;
use crate::{
    input::{LoadedSvgFile, ResolvedGenerateWebfontsOptions},
    rendering::{paths::join_url, shared::to_io_err},
    types::FontType,
};

pub(super) fn make_urls(
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
                options.font_name,
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
                    options.font_name,
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

pub(super) fn make_ctx(
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
    pub css_template_dependencies: TemplateDependencies,
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
        // Default template always uses src. Dynamic custom templates stay URL-sensitive because
        // helpers, partials, or whole-context reads may observe `src` indirectly.
        let css_template_dependencies = match &css_template_source {
            None => TemplateDependencies::css_default(),
            Some(source) => template_dependencies(source),
        };
        let css_template_uses_src = css_template_dependencies.may_depend_on_src();
        let (codepoints_hex, codepoints_num) = make_codepoints(options);
        Ok(Self {
            codepoints_hex,
            codepoints_num,
            css_template_source,
            css_registry_cache: std::sync::OnceLock::new(),
            css_template_dependencies,
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

pub(super) fn resolved_template_options(
    options: &ResolvedGenerateWebfontsOptions,
) -> Map<String, Value> {
    let mut template_options = Map::from_iter([
        ("baseSelector".to_owned(), Value::String(".icon".to_owned())),
        ("classPrefix".to_owned(), Value::String("icon-".to_owned())),
    ]);

    if let Some(custom_template_options) = &options.template_options {
        template_options.extend(custom_template_options.clone());
    }

    template_options
}
