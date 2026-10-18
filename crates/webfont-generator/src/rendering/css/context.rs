use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Error;
use std::sync::Arc;

use handlebars::Handlebars;
use serde_json::{Map, Value};

#[cfg(feature = "fs")]
use super::dependencies::{TemplateDependencies, template_dependencies};
#[cfg(feature = "fs")]
use crate::input::ResolvedFileGenerationOptions;
use crate::{
    input::{ResolvedGenerationOptions, ResolvedVariants},
    rendering::{paths::join_url, shared::to_io_err},
    types::FontType,
};

pub(crate) fn make_urls(
    options: &ResolvedGenerationOptions,
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
    options: &ResolvedGenerationOptions,
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

#[cfg(feature = "fs")]
pub(super) fn make_ctx(
    options: &ResolvedFileGenerationOptions,
    urls: &HashMap<FontType, String>,
    shared: &SharedTemplateData,
) -> Map<String, Value> {
    make_content_css_context(options, options.variants.as_ref(), urls, shared)
}

pub(crate) fn make_content_css_context(
    options: &ResolvedGenerationOptions,
    variants: Option<&ResolvedVariants>,
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
    ctx.insert(
        "colorSlots".to_owned(),
        Value::Object(
            options
                .color_slots
                .keys()
                .enumerate()
                .map(|(index, name)| (name.clone(), Value::from(index as u32)))
                .collect(),
        ),
    );
    // Engine-owned: arbitrary template data must not switch the rendering mode.
    ctx.insert(
        "__webfontVariantMode".to_owned(),
        Value::Bool(variants.is_some()),
    );

    if let Some(variants) = variants {
        ctx.insert(
            "defaultWeight".to_owned(),
            Value::Number(variants.variants[variants.default_index].weight.into()),
        );
        ctx.insert(
            "fontStyle".to_owned(),
            Value::String(options.font_style.as_deref().unwrap_or("normal").to_owned()),
        );
        ctx.insert(
            "variants".to_owned(),
            Value::Array(
                variants
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(index, variant)| {
                        serde_json::json!({"name": variant.name, "weight": variant.weight,
                "default": index == variants.default_index, "className": variant.class_name,
                "selector": variant.selector})
                    })
                    .collect(),
            ),
        );
        let first = &variants.variants[0];
        ctx.insert(
            "variantClassPrefix".to_owned(),
            Value::String(
                first
                    .class_name
                    .strip_suffix(&first.name)
                    .unwrap()
                    .to_owned(),
            ),
        );
    }

    ctx
}

#[cfg(feature = "fs")]
pub(crate) fn build_css_context(
    options: &ResolvedFileGenerationOptions,
    shared: &SharedTemplateData,
) -> Map<String, Value> {
    build_css_context_with_fonts_url(options, shared, options.css_fonts_url.as_deref())
}

#[cfg(feature = "fs")]
pub(crate) fn build_css_context_with_fonts_url(
    options: &ResolvedFileGenerationOptions,
    shared: &SharedTemplateData,
    css_fonts_url: Option<&str>,
) -> Map<String, Value> {
    let urls = make_urls(options, &shared.hash, css_fonts_url);
    make_ctx(options, &urls, shared)
}

/// Pre-computed values shared between CSS and HTML context building.
/// Owns shared template text without acquiring it. Compilation is deferred to the
/// first render via OnceLock; the caller supplies its mode-specific fingerprint.
pub(crate) struct SharedTemplateData {
    pub codepoints_hex: Map<String, Value>,
    pub codepoints_num: Map<String, Value>,
    pub(crate) css_template_source: Option<Arc<str>>,
    css_registry_cache: std::sync::OnceLock<Result<Handlebars<'static>, String>>,
    #[cfg(feature = "fs")]
    pub css_template_dependencies: TemplateDependencies,
    /// Whether the CSS template references `{src}` — if false, URL overrides are a no-op.
    #[cfg(feature = "fs")]
    pub css_template_uses_src: bool,
    pub hash: String,
    pub template_options: Map<String, Value>,
    #[cfg(test)]
    pub(crate) css_compile_count: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    pub(crate) css_render_count: std::sync::atomic::AtomicUsize,
}

impl SharedTemplateData {
    pub fn new(
        options: &ResolvedGenerationOptions,
        css_template_source: Option<Arc<str>>,
        hash: String,
    ) -> Self {
        // Default template always uses src. Dynamic custom templates stay URL-sensitive because
        // helpers, partials, or whole-context reads may observe `src` indirectly.
        #[cfg(feature = "fs")]
        let css_template_dependencies = match &css_template_source {
            None => TemplateDependencies::css_default(),
            Some(source) => template_dependencies(source),
        };
        #[cfg(feature = "fs")]
        let css_template_uses_src = css_template_dependencies.may_depend_on_src();
        let (codepoints_hex, codepoints_num) = make_codepoints(options);
        Self {
            codepoints_hex,
            codepoints_num,
            css_template_source,
            css_registry_cache: std::sync::OnceLock::new(),
            #[cfg(feature = "fs")]
            css_template_dependencies,
            #[cfg(feature = "fs")]
            css_template_uses_src,
            hash,
            template_options: resolved_template_options(options),
            #[cfg(test)]
            css_compile_count: Default::default(),
            #[cfg(test)]
            css_render_count: Default::default(),
        }
    }

    /// Returns the compiled CSS Handlebars registry, compiling on first access.
    /// Returns None when no custom template is configured (default hot path).
    pub fn css_registry(&self) -> Result<Option<&Handlebars<'static>>, Error> {
        match &self.css_template_source {
            None => Ok(None),
            Some(source) => {
                let result = self.css_registry_cache.get_or_init(|| {
                    #[cfg(test)]
                    self.css_compile_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let mut registry = Handlebars::new();
                    handlebars::handlebars_helper!(sass_string: |value: str| value.replace('\\', "\\\\").replace('\'', "\\'"));
                    registry.register_helper("sassString", Box::new(sass_string));
                    registry
                        .register_template_string("css", source.as_ref())
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
    options: &ResolvedGenerationOptions,
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

pub(super) fn resolved_template_options(options: &ResolvedGenerationOptions) -> Map<String, Value> {
    let mut template_options = Map::from_iter([
        ("baseSelector".to_owned(), Value::String(".icon".to_owned())),
        ("classPrefix".to_owned(), Value::String("icon-".to_owned())),
    ]);

    if let Some(custom_template_options) = &options.template_options {
        template_options.extend(custom_template_options.clone());
    }

    template_options
}
