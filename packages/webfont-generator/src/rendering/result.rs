#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::{Map, Value};

use super::css::{
    SharedTemplateData, TemplateDependencies, build_css_context, make_src,
    render_css_with_hbs_context, render_css_with_src_mutate,
};
use super::html::{
    build_html_context, build_html_registry_and_dependencies, render_default_html_with_styles,
    render_html_with_hbs_context,
};
use super::shared::{render_with_field_swap, to_io_err};
use crate::types::{FontType, GenerateWebfontsResult};

/// Caches the last rendered CSS/HTML result for repeated calls with the same urls. Cloneable so
/// an incremental `regenerate` can carry the still-valid entries (provided-URL renders, which
/// don't depend on the font hash) forward into the rebuilt template data.
#[derive(Clone, Default)]
pub(crate) struct RenderCache {
    /// Result of generateCss() with no urls (computed once).
    css_no_urls: Option<String>,
    /// Last generateCss(urls) result.
    css_last_urls: Option<HashMap<FontType, String>>,
    css_last_result: Option<String>,
    /// Result of generateHtml() with no urls (computed once).
    html_no_urls: Option<String>,
    /// Last generateHtml(urls) result.
    html_last_urls: Option<HashMap<FontType, String>>,
    html_last_result: Option<String>,
}

#[derive(Clone)]
pub(crate) struct CarriedRenderCache {
    cache: RenderCache,
    css_dependencies: TemplateDependencies,
    html_dependencies: TemplateDependencies,
}

pub(crate) struct CachedTemplateData {
    pub(crate) shared: SharedTemplateData,
    pub(crate) css_context: Map<String, Value>,
    pub(crate) css_hbs_context: Mutex<handlebars::Context>,
    pub(crate) html_context: Map<String, Value>,
    pub(crate) html_hbs_context: Mutex<handlebars::Context>,
    pub(crate) html_template_dependencies: TemplateDependencies,
    pub(crate) html_registry: Option<handlebars::Handlebars<'static>>,
    pub(crate) render_cache: Mutex<RenderCache>,
}

impl GenerateWebfontsResult {
    #[cfg(test)]
    pub(crate) fn has_carried_css_no_urls_for_test(&self) -> bool {
        self.carried_render
            .as_ref()
            .is_some_and(|carried| carried.cache.css_no_urls.is_some())
    }

    #[cfg(test)]
    pub(crate) fn has_carried_html_no_urls_for_test(&self) -> bool {
        self.carried_render
            .as_ref()
            .is_some_and(|carried| carried.cache.html_no_urls.is_some())
    }

    pub(crate) fn get_cached_io(&self) -> std::io::Result<&CachedTemplateData> {
        self.cached
            .get_or_init(|| {
                let shared = SharedTemplateData::new(&self.options, &self.source_files)
                    .map_err(|e| e.to_string())?;
                let css_context = match &self.css_context {
                    Some(ctx) => ctx.clone(),
                    None => build_css_context(&self.options, &shared),
                };
                let html_context = match &self.html_context {
                    Some(ctx) => ctx.clone(),
                    None => build_html_context(&self.options, &shared, &self.source_files, None)
                        .map_err(|e| e.to_string())?,
                };
                let (html_registry, html_template_dependencies) =
                    build_html_registry_and_dependencies(&self.options)
                        .map_err(|e| e.to_string())?;
                let css_hbs_context =
                    handlebars::Context::wraps(&css_context).map_err(|e| e.to_string())?;
                let html_hbs_context =
                    handlebars::Context::wraps(&html_context).map_err(|e| e.to_string())?;
                Ok(CachedTemplateData {
                    shared,
                    css_context,
                    css_hbs_context: Mutex::new(css_hbs_context),
                    html_context,
                    html_hbs_context: Mutex::new(html_hbs_context),
                    html_template_dependencies,
                    html_registry,
                    // Seed with entries carried across a regenerate;
                    // these are renders that don't depend on what changed, so reusing them is safe.
                    render_cache: Mutex::new(
                        self.carried_render
                            .as_ref()
                            .map(|carried| carried.cache.clone())
                            .unwrap_or_default(),
                    ),
                })
            })
            .as_ref()
            .map_err(to_io_err)
    }

    pub(crate) fn reusable_render_cache(
        &self,
        names_unchanged: bool,
        codepoints_unchanged: bool,
    ) -> Option<CarriedRenderCache> {
        self.render_cache_source().map(|carried| {
            let css_deps = carried.css_dependencies;
            let css_no_urls_unchanged =
                css_deps.can_reuse_css_no_urls(names_unchanged, codepoints_unchanged);
            let css_with_urls_unchanged =
                css_deps.can_reuse_css_with_urls(names_unchanged, codepoints_unchanged);
            let html_no_urls_unchanged = carried.html_dependencies.can_reuse_html(
                names_unchanged,
                codepoints_unchanged,
                css_no_urls_unchanged,
            );
            let html_with_urls_unchanged = carried.html_dependencies.can_reuse_html(
                names_unchanged,
                codepoints_unchanged,
                css_with_urls_unchanged,
            );

            let rc = carried.cache;
            CarriedRenderCache {
                css_dependencies: carried.css_dependencies,
                html_dependencies: carried.html_dependencies,
                cache: RenderCache {
                    css_no_urls: css_no_urls_unchanged
                        .then(|| rc.css_no_urls.clone())
                        .flatten(),
                    html_no_urls: html_no_urls_unchanged
                        .then(|| rc.html_no_urls.clone())
                        .flatten(),
                    css_last_urls: css_with_urls_unchanged
                        .then(|| rc.css_last_urls.clone())
                        .flatten(),
                    css_last_result: css_with_urls_unchanged
                        .then(|| rc.css_last_result.clone())
                        .flatten(),
                    html_last_urls: html_with_urls_unchanged
                        .then(|| rc.html_last_urls.clone())
                        .flatten(),
                    html_last_result: html_with_urls_unchanged
                        .then(|| rc.html_last_result.clone())
                        .flatten(),
                },
            }
        })
    }

    pub(crate) fn render_cache_source(&self) -> Option<CarriedRenderCache> {
        self.cached
            .get()
            .and_then(|cached| cached.as_ref().ok())
            .map(|cached| CarriedRenderCache {
                cache: cached.render_cache.lock().unwrap().clone(),
                css_dependencies: cached.shared.css_template_dependencies,
                html_dependencies: cached.html_template_dependencies,
            })
            .or_else(|| self.carried_render.clone())
    }

    /// Generate a CSS string for this webfont result.
    ///
    /// Pass `urls` to override the default font URLs in the CSS output.
    pub fn generate_css_pure(
        &self,
        urls: Option<HashMap<FontType, String>>,
    ) -> std::io::Result<String> {
        let cached = self.get_cached_io()?;
        let mut rc = cached.render_cache.lock().unwrap();

        match &urls {
            None => {
                if let Some(result) = &rc.css_no_urls {
                    return Ok(result.clone());
                }
                let ctx = cached.css_hbs_context.lock().unwrap();
                let result =
                    render_css_with_hbs_context(&cached.shared, &ctx, &cached.css_context)?;
                rc.css_no_urls = Some(result.clone());
                Ok(result)
            }
            Some(urls) => {
                // If the template doesn't reference {{src}}, URLs don't affect output
                if !cached.shared.css_template_uses_src {
                    drop(rc);
                    return self.generate_css_pure(None);
                }
                if rc.css_last_urls.as_ref() == Some(urls)
                    && let Some(result) = &rc.css_last_result
                {
                    return Ok(result.clone());
                }
                let src = make_src(&self.options, urls);
                let mut ctx = cached.css_hbs_context.lock().unwrap();
                let result = render_css_with_src_mutate(
                    &cached.shared,
                    &mut ctx,
                    &cached.css_context,
                    &src,
                )?;
                rc.css_last_urls = Some(urls.clone());
                rc.css_last_result = Some(result.clone());
                Ok(result)
            }
        }
    }

    /// Generate an HTML string for this webfont result.
    ///
    /// Pass `urls` to override the default font URLs in the HTML output.
    pub fn generate_html_pure(
        &self,
        urls: Option<HashMap<FontType, String>>,
    ) -> std::io::Result<String> {
        let cached = self.get_cached_io()?;
        let mut rc = cached.render_cache.lock().unwrap();

        match &urls {
            None => {
                if let Some(result) = &rc.html_no_urls {
                    return Ok(result.clone());
                }
                let ctx = cached.html_hbs_context.lock().unwrap();
                let result = render_html_with_hbs_context(
                    cached.html_registry.as_ref(),
                    &ctx,
                    &cached.html_context,
                )?;
                rc.html_no_urls = Some(result.clone());
                Ok(result)
            }
            Some(urls) => {
                // If the CSS template doesn't reference {{src}}, URLs don't affect output
                if !cached.shared.css_template_uses_src {
                    drop(rc);
                    return self.generate_html_pure(None);
                }
                if rc.html_last_urls.as_ref() == Some(urls)
                    && let Some(result) = &rc.html_last_result
                {
                    return Ok(result.clone());
                }
                // Render CSS with the custom URLs (in-place src mutate, no clone)
                let src = make_src(&self.options, urls);
                let styles = {
                    let mut css_ctx = cached.css_hbs_context.lock().unwrap();
                    render_css_with_src_mutate(
                        &cached.shared,
                        &mut css_ctx,
                        &cached.css_context,
                        &src,
                    )?
                };
                // Hot path: default HTML template -- inject styles directly, skip clone
                if self.options.html_template.is_none() {
                    let result = render_default_html_with_styles(&cached.html_context, &styles);
                    rc.html_last_urls = Some(urls.clone());
                    rc.html_last_result = Some(result.clone());
                    return Ok(result);
                }
                // Custom HTML template: in-place styles mutate, no clone
                let mut html_ctx = cached.html_hbs_context.lock().unwrap();
                let registry = cached
                    .html_registry
                    .as_ref()
                    .expect("HTML registry should exist for custom template");
                let result = render_with_field_swap(
                    &mut html_ctx,
                    "styles",
                    serde_json::Value::String(styles),
                    |ctx| registry.render_with_context("html", ctx).map_err(to_io_err),
                )?;
                rc.html_last_urls = Some(urls.clone());
                rc.html_last_result = Some(result.clone());
                Ok(result)
            }
        }
    }
}
