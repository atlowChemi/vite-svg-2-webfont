mod options;
#[cfg(test)]
mod tests;
mod updates;

use std::io::{Error, Result};
use std::sync::{Arc, OnceLock};

use crate::cache::{CacheState, glyph_set, variant_glyph_set};
use crate::incremental::sources::{SourceUpdate, refresh_source_cache, replace_source_cache};
use crate::input::finalize_generate_webfonts_options;
use crate::output::collect::{CollectedOutput, collect_outputs};
use crate::pipeline::{
    TtfGlyphCache, build_content_variant_outputs, build_font_outputs, build_variant_with_cache,
    build_with_cache,
};
use crate::rendering::css::{
    SharedTemplateData, calc_content_hash, make_content_css_context, make_urls,
    render_css_with_context,
};
use crate::rendering::html::{
    compile_html_template, make_content_html_context, render_html_with_hbs_context,
};
use crate::resolve_content_variant_sources;
use crate::result::FontOutputs;
use crate::svg::types::GlyphCache;
use crate::svg::{
    VariantGlyphCache, prepare_svg_font, prepare_svg_font_incremental,
    prepare_variant_svg_family_cached, source_content_hash, svg_options_from_options,
};
pub use options::{GenerateWebfontsFromSourcesOptions, SourceFontVariant, SvgSource};
use options::{ResolvedSourceOptions, resolve};
#[cfg(feature = "async")]
pub use updates::SourceRegenerateError;
pub use updates::{SourceChange, SourceMembership, VariantSourceSet};

/// Persistent cache identity and glyph data retained so `cache_bytes` can encode on request.
struct PersistentCache {
    state: CacheState,
    /// Glyph data for results without incremental caches (which hold it otherwise).
    glyphs: Option<SourceCaches>,
    /// Set when a failed update reset the incremental caches: until a build succeeds and refills
    /// them, exporting would drop every saved glyph, so `cache_bytes` returns `None` instead.
    caches_reset: bool,
}

#[derive(Default)]
struct SourceCaches {
    glyphs: GlyphCache,
    ttf: TtfGlyphCache,
    variants: VariantGlyphCache,
}

/// Font buffers and independently lazy companion renders from SVG text.
/// No method on this result reads or writes files. Settings and templates stay fixed.
pub struct GenerateWebfontsFromSourcesResult {
    options: ResolvedSourceOptions,
    fonts: FontOutputs,
    caches: Option<SourceCaches>,
    persistent: Option<PersistentCache>,
    shared: OnceLock<SharedTemplateData>,
    css: OnceLock<std::result::Result<Arc<String>, String>>,
    html: OnceLock<std::result::Result<Arc<String>, String>>,
    ts: OnceLock<Arc<String>>,
    #[cfg(test)]
    html_render_count: std::sync::atomic::AtomicUsize,
}

/// Generate fonts directly from named SVG text, without a runtime or filesystem access.
/// Both ordinary and variant builds retain the engine's Rayon parallelism.
///
/// ```
/// use webfont_generator::{generate_from_sources_sync, GenerateWebfontsFromSourcesOptions,
///     SvgSource, SourceChange, SourceMembership, FontType};
/// let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>"#;
/// let mut result = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
///     sources: vec![SvgSource { id: "42".into(), name: "square".into(), svg: svg.into() }],
///     types: Some(vec![FontType::Ttf]),
///     incremental: Some(true),
///     ..Default::default()
/// })?;
/// assert!(result.ttf_bytes().is_some());
/// result.regenerate_sync(SourceMembership::Sources(vec!["42".into()]), vec![
///     SourceChange::Changed { id: "42".into(), svg: svg.into(), name: Some("renamed".into()) },
/// ])?;
/// let outputs = result.collect_outputs()?;
/// assert_eq!(outputs.len(), 2); // TTF and CSS; both shared with the result.
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn generate_from_sources_sync(
    options: GenerateWebfontsFromSourcesOptions,
) -> Result<GenerateWebfontsFromSourcesResult> {
    let threads = options.threads;
    crate::threads::install(threads, || generate_in_pool(options))
}

fn generate_in_pool(
    mut options: GenerateWebfontsFromSourcesOptions,
) -> Result<GenerateWebfontsFromSourcesResult> {
    let cache = options.cache.take();
    let mut options = resolve(options)?;
    let mut caches = options.common.incremental.then(SourceCaches::default);
    let (fonts, persistent) = match cache {
        Some(bytes) => build_cached(&mut options, caches.as_mut(), bytes)?,
        None => (build(&mut options, caches.as_mut(), &[])?, None),
    };
    Ok(GenerateWebfontsFromSourcesResult {
        options,
        fonts,
        caches,
        persistent,
        shared: OnceLock::new(),
        css: OnceLock::new(),
        html: OnceLock::new(),
        ts: OnceLock::new(),
        #[cfg(test)]
        html_render_count: Default::default(),
    })
}

/// Run [`generate_from_sources_sync`] on Tokio's blocking pool.
/// Requires an active Tokio runtime; scheduling/join failures are returned as errors.
#[cfg(feature = "async")]
pub async fn generate_from_sources(
    options: GenerateWebfontsFromSourcesOptions,
) -> Result<GenerateWebfontsFromSourcesResult> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|error| {
        Error::other(format!(
            "Source generation requires a Tokio runtime: {error}"
        ))
    })?;
    runtime
        .spawn_blocking(move || generate_from_sources_sync(options))
        .await
        .map_err(generation_join_error)?
}

#[cfg(feature = "async")]
fn generation_join_error(error: tokio::task::JoinError) -> Error {
    Error::other(format!("Source generation task failed: {error}"))
}

/// Initial build seeded from persistent cache bytes. The persistent state is dropped when nothing
/// could ever be exported: a full hit without incremental updates.
fn build_cached(
    options: &mut ResolvedSourceOptions,
    caches: Option<&mut SourceCaches>,
    bytes: Vec<u8>,
) -> Result<(FontOutputs, Option<PersistentCache>)> {
    if let Some(variants) = &options.variants {
        let family = resolve_content_variant_sources(
            &mut options.common,
            variants,
            options.sources.iter().map(Vec::as_slice).collect(),
        )?;
        let mut state = CacheState::new_variant(&options.common, variants, &family);
        if !bytes.is_empty() {
            state.load(bytes);
        }
        let svg_options = svg_options_from_options(&options.common);
        let built =
            build_variant_with_cache(&options.common, variants, &svg_options, &family, &state);
        let (fonts, glyphs, _) = built?;
        let glyphs = match (caches, glyphs) {
            (Some(cache), Some(glyphs)) => {
                cache.variants = glyphs;
                None
            }
            (None, None) => return Ok((fonts, None)),
            (_, glyphs) => glyphs.map(|variants| SourceCaches {
                variants,
                ..Default::default()
            }),
        };
        state.release_glyphs();
        let persistent = PersistentCache {
            state,
            glyphs,
            caches_reset: false,
        };
        return Ok((fonts, Some(persistent)));
    }
    finalize_generate_webfonts_options(&mut options.common, &options.sources[0])?;
    let sources = &options.sources[0];
    let mut state = CacheState::new(&options.common, sources);
    if !bytes.is_empty() {
        state.load(bytes);
    }
    let svg_options = svg_options_from_options(&options.common);
    let mut caches = caches;
    let ttf = caches.as_deref_mut().map(|cache| &mut cache.ttf);
    let built = build_with_cache(&options.common, &svg_options, sources, &state, ttf);
    let (fonts, glyphs, _) = built?;
    // Incremental results keep glyph data in their caches; others retain it for export.
    let glyphs = match (caches, glyphs) {
        (Some(cache), Some(glyphs)) => {
            cache.glyphs = glyphs;
            None
        }
        (None, None) => return Ok((fonts, None)),
        (_, glyphs) => glyphs.map(|glyphs| SourceCaches {
            glyphs,
            ..Default::default()
        }),
    };
    state.release_glyphs();
    let persistent = PersistentCache {
        state,
        glyphs,
        caches_reset: false,
    };
    Ok((fonts, Some(persistent)))
}

fn build(
    options: &mut ResolvedSourceOptions,
    mut caches: Option<&mut SourceCaches>,
    updates: &[SourceUpdate],
) -> Result<FontOutputs> {
    // Finish all membership, codepoint, color and missing-glyph validation before touching caches.
    let family = if let Some(variants) = &options.variants {
        Some(resolve_content_variant_sources(
            &mut options.common,
            variants,
            options.sources.iter().map(Vec::as_slice).collect(),
        )?)
    } else {
        finalize_generate_webfonts_options(&mut options.common, &options.sources[0])?;
        None
    };
    if family.is_none()
        && let Some(cache) = caches.as_mut()
    {
        for update in updates {
            match update {
                SourceUpdate::Set(source) => {
                    refresh_source_cache(
                        &mut cache.glyphs,
                        &source.id,
                        source_content_hash(&source.contents),
                    );
                }
                SourceUpdate::Remove(id) => replace_source_cache(&mut cache.glyphs, id, None),
            }
        }
    }
    let outputs = (|| {
        let svg_options = svg_options_from_options(&options.common);
        if let Some(family) = family {
            let prepared = prepare_variant_svg_family_cached(
                &svg_options,
                &family,
                caches.as_deref_mut().map(|cache| &mut cache.variants),
            )?;
            return build_content_variant_outputs(
                &options.common,
                options.variants.as_ref().unwrap(),
                &prepared,
            );
        }
        let sources = &options.sources[0];
        match caches.as_deref_mut() {
            Some(cache) => {
                let prepared =
                    prepare_svg_font_incremental(&svg_options, sources, &mut cache.glyphs)?;
                build_font_outputs(
                    &options.common,
                    &svg_options,
                    &prepared,
                    Some(&mut cache.ttf),
                )
            }
            None => {
                let prepared = prepare_svg_font(&svg_options, sources)?;
                build_font_outputs(&options.common, &svg_options, &prepared, None)
            }
        }
    })();
    if outputs.is_err()
        && let Some(cache) = caches
    {
        *cache = SourceCaches::default();
    }
    outputs
}

impl GenerateWebfontsFromSourcesResult {
    /// Borrow the requested SVG font.
    pub fn svg_string(&self) -> Option<&str> {
        self.fonts.svg_font.as_deref().map(String::as_str)
    }
    /// Borrow the requested TTF font.
    pub fn ttf_bytes(&self) -> Option<&[u8]> {
        self.fonts.ttf_font.as_deref().map(Vec::as_slice)
    }
    /// Borrow the requested EOT font.
    pub fn eot_bytes(&self) -> Option<&[u8]> {
        self.fonts.eot_font.as_deref().map(Vec::as_slice)
    }
    /// Borrow the requested WOFF font.
    pub fn woff_bytes(&self) -> Option<&[u8]> {
        self.fonts.woff_font.as_deref().map(Vec::as_slice)
    }
    /// Borrow the requested WOFF2 font.
    pub fn woff2_bytes(&self) -> Option<&[u8]> {
        self.fonts.woff2_font.as_deref().map(Vec::as_slice)
    }

    fn shared(&self) -> &SharedTemplateData {
        self.shared.get_or_init(|| {
            let sources = self
                .options
                .sources
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            SharedTemplateData::new(
                &self.options.common,
                self.options.css_template.clone(),
                calc_content_hash(
                    &self.options.common,
                    self.options.variants.as_ref(),
                    &sources,
                    self.options.css_template.as_deref(),
                    self.options.html_template.as_deref(),
                ),
            )
        })
    }

    fn css_context(&self) -> serde_json::Map<String, serde_json::Value> {
        let options = &self.options.common;
        let shared = self.shared();
        make_content_css_context(
            options,
            self.options.variants.as_ref(),
            &make_urls(options, &shared.hash, options.css_fonts_url.as_deref()),
            shared,
        )
    }

    /// Render CSS on first request and share the cached string thereafter.
    /// Explicit requests render even when CSS collection is disabled.
    pub fn generate_css(&self) -> Result<Arc<String>> {
        self.css
            .get_or_init(|| {
                render_css_with_context(self.shared(), &self.css_context())
                    .map(Arc::new)
                    .map_err(|error| error.to_string())
            })
            .clone()
            .map_err(Error::other)
    }

    /// Render an HTML preview on first request. Its styles reuse the CSS render cache.
    /// Requesting CSS alone never compiles the HTML template.
    pub fn generate_html(&self) -> Result<Arc<String>> {
        self.html
            .get_or_init(|| {
                self.render_html()
                    .map(Arc::new)
                    .map_err(|error| error.to_string())
            })
            .clone()
            .map_err(Error::other)
    }

    fn render_html(&self) -> Result<String> {
        #[cfg(test)]
        self.html_render_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let template = compile_html_template(self.options.html_template.clone())?;
        let css = self.generate_css()?;
        let sources = self
            .options
            .sources
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let css_ctx = self.options.variants.as_ref().map(|_| self.css_context());
        let context = make_content_html_context(
            &self.options.common,
            self.shared(),
            &sources,
            css.as_ref().clone(),
            css_ctx.as_ref(),
        );
        let hbs_context = handlebars::Context::wraps(&context).map_err(Error::other)?;
        render_html_with_hbs_context(template.registry.as_ref(), &hbs_context, &context)
    }

    /// Render a fixed TypeScript source module with icon names and selector settings.
    /// Explicit requests work even when TS collection is disabled and never render templates.
    pub fn generate_ts(&self) -> Result<Arc<String>> {
        Ok(self
            .ts
            .get_or_init(|| {
                Arc::new(crate::rendering::ts::render_ts(
                    &self.options.common,
                    self.options
                        .sources
                        .iter()
                        .flatten()
                        .map(|source| source.glyph_name.as_str()),
                    self.options.variants.is_some(),
                    None,
                ))
            })
            .clone())
    }

    /// Encode the persistent cache for storage, reflecting the current fonts and sources
    /// (including incremental updates). Returns `None` when the result was generated without
    /// the [`cache`](GenerateWebfontsFromSourcesOptions::cache) option, or when the bytes equal
    /// the ones passed in or last returned, so callers can skip rewriting their store.
    pub fn cache_bytes(&mut self) -> Option<Vec<u8>> {
        let PersistentCache {
            state,
            glyphs,
            caches_reset,
        } = self.persistent.as_mut()?;
        if *caches_reset {
            return None;
        }
        let caches = glyphs.as_ref().or(self.caches.as_ref())?;
        let options = &self.options.common;
        if let Some(variants) = &self.options.variants {
            let sources = &self.options.sources;
            let family = resolve_content_variant_sources(
                &mut options.clone(),
                variants,
                sources.iter().map(Vec::as_slice).collect(),
            )
            .expect("committed variant sources resolve");
            state.refresh_glyph_set(variant_glyph_set(&family));
            return state.update_variants(sources, &caches.variants, &self.fonts, &options.types);
        }
        let sources = &self.options.sources[0];
        state.refresh_glyph_set(glyph_set(options, sources));
        state.update(sources, &caches.glyphs, &self.fonts, &options.types)
    }

    /// Collect fonts and enabled companions, sharing all payloads with this result.
    pub fn collect_outputs(&self) -> Result<Vec<CollectedOutput>> {
        let css = self
            .options
            .common
            .css
            .then(|| self.generate_css())
            .transpose()?;
        let html = self
            .options
            .common
            .html
            .then(|| self.generate_html())
            .transpose()?;
        let ts = self
            .options
            .common
            .ts
            .then(|| self.generate_ts())
            .transpose()?;
        Ok(collect_outputs(&self.fonts, css, html, ts))
    }
}
