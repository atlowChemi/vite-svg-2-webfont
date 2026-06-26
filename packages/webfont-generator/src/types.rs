use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "napi")]
use napi::bindgen_prelude::Uint8Array;
#[cfg(feature = "napi")]
use napi_derive::napi;
use serde_json::{Map, Value};

use crate::svg::types::GlyphCache;
use crate::templates::{
    SharedTemplateData, TemplateDependencies, build_css_context, build_html_context,
    build_html_registry_and_dependencies, make_src, render_css_with_hbs_context,
    render_css_with_src_mutate, render_default_html_with_styles, render_html_with_hbs_context,
};
use crate::ttf::TtfGlyphCache;
use crate::util::to_io_err;

/// What happened to a file, for [`GenerateWebfontsResult::regenerate`]. `name` is the
/// caller-resolved glyph name (the `rename` callback, if any, is applied by the caller).
pub enum GlyphChange {
    /// A new file. `name` overrides the file-stem glyph name when `Some`.
    Added { name: Option<String> },
    /// An existing file's contents changed. `name` overrides the glyph name when `Some`.
    Changed { name: Option<String> },
    /// The file was deleted.
    Removed,
}

/// One entry in the `changes` array passed to the Node binding's `regenerate`. The complete
/// ordered file list passed alongside it controls final glyph order; this only describes which
/// files need re-reading, renaming, or removal.
#[cfg_attr(feature = "napi", napi(object))]
pub struct GlyphChangeEntry {
    /// Path of the changed file.
    pub path: String,
    /// What happened to the file.
    #[cfg_attr(feature = "napi", napi(ts_type = "'added' | 'changed' | 'removed'"))]
    pub change_type: String,
    /// Resolved glyph name (with the caller's `rename` already applied). Optional for `'added'`
    /// and `'changed'` (defaults to the file stem/current name); ignored for `'removed'`.
    pub name: Option<String>,
}

/// Font output format. Used in the `types` and `order` options to control which
/// formats are generated and the order they appear in the CSS `@font-face`
/// `src:` descriptor.
#[cfg_attr(feature = "napi", napi(string_enum = "lowercase"))]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontType {
    /// SVG font (`.svg`). Legacy format; intermediate representation that all
    /// other formats are derived from.
    Svg,
    /// TrueType font (`.ttf`).
    Ttf,
    /// Embedded OpenType (`.eot`). Legacy format for older Internet Explorer.
    Eot,
    /// Web Open Font Format 1.0 (`.woff`).
    Woff,
    /// Web Open Font Format 2.0 (`.woff2`). Best compression; preferred for
    /// modern browsers.
    Woff2,
}

impl FontType {
    /// Returns the CSS `format()` value (e.g., "truetype", "woff2").
    #[inline]
    pub fn css_format(self) -> &'static str {
        match self {
            FontType::Svg => "svg",
            FontType::Ttf => "truetype",
            FontType::Eot => "embedded-opentype",
            FontType::Woff => "woff",
            FontType::Woff2 => "woff2",
        }
    }

    /// Returns the file extension (e.g., "svg", "ttf", "woff2").
    #[inline]
    pub fn as_extension(self) -> &'static str {
        match self {
            FontType::Svg => "svg",
            FontType::Ttf => "ttf",
            FontType::Eot => "eot",
            FontType::Woff => "woff",
            FontType::Woff2 => "woff2",
        }
    }
}

/// SVG-format–specific options for the intermediate SVG font and the per-glyph
/// path processing that feeds every other format.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Default)]
pub struct SvgFormatOptions {
    /// SVG-format override of the top-level `centerVertically` option. When set,
    /// it wins over the top-level value; centers each glyph vertically inside
    /// the em-square based on its bounding box.
    pub center_vertically: Option<bool>,
    /// Value of the SVG font's `id` attribute. Defaults to `fontName` when
    /// omitted.
    pub font_id: Option<String>,
    /// Content embedded inside the generated SVG font's `<metadata>` element.
    pub metadata: Option<String>,
    /// SVG-format override of the top-level `optimizeOutput` option. When set,
    /// it wins over the top-level value; runs an SVG path optimizer over each
    /// glyph, trading a small amount of build time for smaller output bytes.
    pub optimize_output: Option<bool>,
    /// SVG-format override of the top-level `preserveAspectRatio` option. When
    /// set, it wins over the top-level value; preserves the source viewBox
    /// aspect ratio when scaling glyphs into the em-square.
    pub preserve_aspect_ratio: Option<bool>,
}

/// TTF-format–specific options. Populates fields in the generated TTF `name`
/// and `head` tables.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct TtfFormatOptions {
    /// Copyright string written to the TTF `name` table (record id 0).
    pub copyright: Option<String>,
    /// Description string written to the TTF `name` table (record id 10).
    pub description: Option<String>,
    /// Unix timestamp in seconds used for the `created` and `modified` fields
    /// in the TTF `head` table. Pin to a fixed value to produce byte-stable
    /// reproducible builds.
    pub ts: Option<i64>,
    /// Manufacturer URL written to the TTF `name` table (record id 11).
    pub url: Option<String>,
    /// Version string written to the TTF `name` table (record id 5).
    pub version: Option<String>,
}

/// WOFF-format–specific options. Affects only WOFF1 output; WOFF2 ignores these.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct WoffFormatOptions {
    /// XML string embedded in the WOFF1 metadata block.
    pub metadata: Option<String>,
}

/// WOFF2-format–specific options. Affects only WOFF2 output.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct Woff2FormatOptions {
    /// Brotli compression quality used when encoding WOFF2, from `0` (fastest,
    /// largest output) to `11` (slowest, smallest output). This tunes compression
    /// effort only and never changes glyph fidelity — the decompressed font is
    /// identical at every level. Defaults to `11` for the smallest output; lower it
    /// (e.g. to `10`) for faster encoding at a marginal size cost. Must be between
    /// `0` and `11`; other values are rejected.
    pub compression_quality: Option<u8>,
}

/// Per-format configuration object. Each field carries options that only apply
/// to the corresponding output format.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Default)]
pub struct FormatOptions {
    /// SVG-format options.
    pub svg: Option<SvgFormatOptions>,
    /// TTF-format options.
    pub ttf: Option<TtfFormatOptions>,
    /// WOFF1-format options.
    pub woff: Option<WoffFormatOptions>,
    /// WOFF2-format options.
    pub woff2: Option<Woff2FormatOptions>,
}

/// Guaranteed fields supplied to a `cssContext` callback. Additional keys from
/// user-supplied `templateOptions` are merged into the same object at runtime,
/// so the JS-side type widens this with an open-ended index signature.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct CssContext {
    /// Name of the generated font, mirroring the `fontName` option.
    pub font_name: String,
    /// Pre-rendered value for the CSS `@font-face` `src:` descriptor — a
    /// comma-separated list of `url(...) format(...)` entries derived from the
    /// configured `types`, `order`, and `cssFontsUrl`.
    pub src: String,
    /// Map from glyph name to its assigned codepoint as a hex-encoded string
    /// (e.g. `"add" -> "f101"`), suitable for use inside CSS `content`
    /// declarations like `content: "\f101"`.
    pub codepoints: HashMap<String, String>,
}

/// Guaranteed fields supplied to an `htmlContext` callback. Additional keys
/// from user-supplied `templateOptions` are merged into the same object at
/// runtime, so the JS-side type widens this with an open-ended index signature.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct HtmlContext {
    /// Name of the generated font, mirroring the `fontName` option.
    pub font_name: String,
    /// Glyph names in declaration order, after any `rename` callback has been
    /// applied. Useful for iterating over icons in a preview template.
    pub names: Vec<String>,
    /// Pre-rendered CSS (the same string the engine writes to the `.css`
    /// output) so HTML templates can embed it inline for self-contained
    /// previews without an external stylesheet reference.
    pub styles: String,
    /// Map from glyph name to its assigned codepoint as a numeric value
    /// (e.g. `"add" -> 0xF101`). Use the CSS context's hex form if you need a
    /// string for embedding into CSS `content` declarations.
    pub codepoints: HashMap<String, u32>,
}

/// Top-level options controlling webfont generation. Only `dest` and `files`
/// are required; every other field has a sensible default. See the per-field
/// docs for defaults and units.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Default)]
pub struct GenerateWebfontsOptions {
    /// Font ascent in font units. Overrides the value computed from the source
    /// glyphs.
    pub ascent: Option<f64>,
    /// When `true`, centers each glyph horizontally inside the em-square based
    /// on its bounding box.
    pub center_horizontally: Option<bool>,
    /// When `true`, centers each glyph vertically inside the em-square based
    /// on its bounding box. Convenience alias for
    /// `formatOptions.svg.centerVertically`.
    pub center_vertically: Option<bool>,
    /// Whether to generate a CSS file. Defaults to `true`.
    pub css: Option<bool>,
    /// Output path for the generated CSS file. Defaults to
    /// `path.join(dest, fontName + '.css')`.
    pub css_dest: Option<String>,
    /// Path to a custom Handlebars template for CSS generation. The template
    /// receives the `cssContext` shape plus any `templateOptions` keys.
    pub css_template: Option<String>,
    /// Explicit Unicode codepoints for specific glyphs, keyed by glyph name.
    /// Glyphs not listed here are auto-assigned starting at `startCodepoint`.
    pub codepoints: Option<HashMap<String, u32>>,
    /// URL prefix for font files in the generated CSS. Defaults to the
    /// relative path from `cssDest` to `dest`.
    pub css_fonts_url: Option<String>,
    /// Font descent in font units. Overrides the value computed from the
    /// source glyphs.
    pub descent: Option<f64>,
    /// Output directory for generated font files. Required.
    pub dest: String,
    /// Paths to the SVG files to include in the font. Required.
    pub files: Vec<String>,
    /// When `true`, produces a monospace font sized to the widest glyph.
    pub fixed_width: Option<bool>,
    /// Per-format option overrides. See `FormatOptions`.
    pub format_options: Option<FormatOptions>,
    /// Whether to generate an HTML preview file. Defaults to `false`.
    pub html: Option<bool>,
    /// Output path for the generated HTML preview file. Defaults to
    /// `path.join(dest, fontName + '.html')`.
    pub html_dest: Option<String>,
    /// Path to a custom Handlebars template for HTML preview generation.
    pub html_template: Option<String>,
    /// Retain parsed glyph data on the result so `regenerate` can rebuild after file changes
    /// without re-parsing unchanged glyphs. Defaults to `false`; enable for watch/dev. One-shot
    /// builds (CLI, production) should leave it off to avoid holding the parsed geometry in memory.
    pub incremental: Option<bool>,
    /// Explicit output font height in units per em. Overrides the height
    /// computed from the source glyphs.
    pub font_height: Option<f64>,
    /// Name of the generated font family; also used as the base name for
    /// output files. Defaults to `'iconfont'`.
    pub font_name: Option<String>,
    /// CSS `font-style` value for the generated `@font-face` rule.
    pub font_style: Option<String>,
    /// CSS `font-weight` value for the generated `@font-face` rule.
    pub font_weight: Option<String>,
    /// Enable ligature support so each glyph can be referenced by its name as
    /// a text ligature. Defaults to `true`.
    pub ligature: Option<bool>,
    /// Scale icons to the height of the tallest icon. Defaults to `true`.
    pub normalize: Option<bool>,
    /// Order of `@font-face` `src:` entries in the generated CSS. Every entry
    /// must also appear in `types`. Defaults to
    /// `['eot', 'woff2', 'woff', 'ttf', 'svg']` filtered to the requested
    /// `types`.
    pub order: Option<Vec<FontType>>,
    /// Run an SVG path optimizer over each glyph, trading a small amount of
    /// build time for smaller output bytes. Convenience alias for
    /// `formatOptions.svg.optimizeOutput`.
    pub optimize_output: Option<bool>,
    /// Preserve the source viewBox aspect ratio when scaling glyphs into the
    /// em-square. Convenience alias for `formatOptions.svg.preserveAspectRatio`.
    pub preserve_aspect_ratio: Option<bool>,
    /// SVG path coordinate rounding precision.
    pub round: Option<f64>,
    /// Starting codepoint for auto-assigned glyphs. Defaults to `0xF101`.
    pub start_codepoint: Option<u32>,
    /// Additional key-value pairs merged into the Handlebars template
    /// context for both CSS and HTML rendering. Typical home for
    /// `classPrefix` and `baseSelector`.
    pub template_options: Option<Map<String, Value>>,
    /// Font formats to generate. Defaults to `['eot', 'woff', 'woff2']`.
    pub types: Option<Vec<FontType>>,
    /// Whether to write generated files to disk. Set to `false` for
    /// in-memory usage. Defaults to `true`.
    pub write_files: Option<bool>,
}

pub(crate) const DEFAULT_FONT_TYPES: [FontType; 3] =
    [FontType::Eot, FontType::Woff, FontType::Woff2];

pub(crate) const DEFAULT_FONT_ORDER: [FontType; 5] = [
    FontType::Eot,
    FontType::Woff2,
    FontType::Woff,
    FontType::Ttf,
    FontType::Svg,
];

pub(crate) fn resolved_font_types(options: &GenerateWebfontsOptions) -> Vec<FontType> {
    match &options.types {
        Some(types) => types.clone(),
        None => DEFAULT_FONT_TYPES.to_vec(),
    }
}

#[derive(Clone)]
pub(crate) struct ResolvedGenerateWebfontsOptions {
    pub ascent: Option<f64>,
    pub center_horizontally: Option<bool>,
    pub center_vertically: Option<bool>,
    pub css: bool,
    pub css_dest: String,
    pub css_template: Option<String>,
    /// Fully-resolved codepoints for the current glyph set (explicit + auto-assigned). Rebuilt
    /// by `finalize_generate_webfonts_options` from `explicit_codepoints` whenever the set changes.
    pub codepoints: BTreeMap<String, u32>,
    /// The user-supplied codepoints, kept as the stable base so re-resolving after an
    /// incremental add/remove assigns the same codepoints a fresh build would.
    pub explicit_codepoints: BTreeMap<String, u32>,
    pub css_fonts_url: Option<String>,
    pub descent: Option<f64>,
    pub dest: String,
    pub files: Vec<String>,
    pub fixed_width: Option<bool>,
    pub format_options: Option<FormatOptions>,
    pub html: bool,
    pub incremental: bool,
    pub html_dest: String,
    pub html_template: Option<String>,
    pub font_height: Option<f64>,
    pub font_name: String,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub ligature: bool,
    pub normalize: bool,
    pub order: Vec<FontType>,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
    pub round: Option<f64>,
    pub start_codepoint: u32,
    pub template_options: Option<Map<String, Value>>,
    pub types: Vec<FontType>,
    pub write_files: bool,
}

#[derive(Clone)]
pub(crate) struct LoadedSvgFile {
    pub contents: String,
    pub glyph_name: String,
    pub path: String,
}

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

pub(crate) struct CachedTemplateData {
    pub shared: SharedTemplateData,
    pub css_context: Map<String, Value>,
    pub css_hbs_context: Mutex<handlebars::Context>,
    pub html_context: Map<String, Value>,
    pub html_hbs_context: Mutex<handlebars::Context>,
    pub html_template_dependencies: TemplateDependencies,
    pub html_registry: Option<handlebars::Handlebars<'static>>,
    pub(crate) render_cache: Mutex<RenderCache>,
}

/// Rendered bytes for each requested output format. Held by [`GenerateWebfontsResult`] and
/// produced by the generator's format pipeline; grouping them lets an incremental regenerate
/// refresh every format in a single assignment.
#[derive(Default)]
pub(crate) struct FontOutputs {
    pub(crate) svg_font: Option<Arc<String>>,
    pub(crate) ttf_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff2_font: Option<Arc<Vec<u8>>>,
    pub(crate) eot_font: Option<Arc<Vec<u8>>>,
}

/// Result of a successful `generateWebfonts` call. Exposes the generated
/// font bytes (or `null` for formats that were not requested) and methods to
/// render the CSS and HTML preview.
#[cfg_attr(feature = "napi", napi)]
pub struct GenerateWebfontsResult {
    pub(crate) cached: OnceLock<Result<CachedTemplateData, String>>,
    /// Render-cache entries carried across an incremental `regenerate` (set by
    /// [`reset_render_cache`]) to seed the rebuilt [`CachedTemplateData`], so CSS/HTML that
    /// doesn't depend on what changed isn't re-rendered. `None` for a normal build.
    pub(crate) carried_render: Option<RenderCache>,
    pub(crate) css_context: Option<Map<String, Value>>,
    pub(crate) fonts: FontOutputs,
    /// Parsed-glyph cache for incremental `regenerate`; `Some` only when `incremental` is set.
    pub(crate) glyph_cache: Option<GlyphCache>,
    pub(crate) html_context: Option<Map<String, Value>>,
    pub(crate) options: ResolvedGenerateWebfontsOptions,
    pub(crate) source_files: Vec<LoadedSvgFile>,
    /// Compiled TTF outline cache for incremental TTF-derived output rebuilds.
    pub(crate) ttf_cache: Option<TtfGlyphCache>,
    /// Hash per CSS/HTML output path of what was last written to disk. Seeded by the initial write
    /// when `write_files` is enabled, then updated by incremental `regenerate` writes so unchanged
    /// rendered companion files are not rewritten. Font outputs are written directly after real
    /// rebuilds because they almost always change and hashing them is slower than writing them.
    pub(crate) written_outputs: HashMap<String, [u8; 16]>,
}

// Pure Rust getters (always available)
impl GenerateWebfontsResult {
    #[cfg(test)]
    pub(crate) fn has_carried_css_no_urls_for_test(&self) -> bool {
        self.carried_render
            .as_ref()
            .is_some_and(|cache| cache.css_no_urls.is_some())
    }

    #[cfg(test)]
    pub(crate) fn has_carried_html_no_urls_for_test(&self) -> bool {
        self.carried_render
            .as_ref()
            .is_some_and(|cache| cache.html_no_urls.is_some())
    }

    /// Returns the EOT font bytes, if generated.
    pub fn eot_bytes(&self) -> Option<&[u8]> {
        self.fonts.eot_font.as_ref().map(|v| v.as_ref().as_slice())
    }

    /// Returns the SVG font string, if generated.
    pub fn svg_string(&self) -> Option<&str> {
        self.fonts.svg_font.as_ref().map(|v| v.as_ref().as_str())
    }

    /// Returns the TTF font bytes, if generated.
    pub fn ttf_bytes(&self) -> Option<&[u8]> {
        self.fonts.ttf_font.as_ref().map(|v| v.as_ref().as_slice())
    }

    /// Returns the WOFF font bytes, if generated.
    pub fn woff_bytes(&self) -> Option<&[u8]> {
        self.fonts.woff_font.as_ref().map(|v| v.as_ref().as_slice())
    }

    /// Returns the WOFF2 font bytes, if generated.
    pub fn woff2_bytes(&self) -> Option<&[u8]> {
        self.fonts
            .woff2_font
            .as_ref()
            .map(|v| v.as_ref().as_slice())
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
                    // Seed with any entries carried across a regenerate (see reset_render_cache);
                    // these are renders that don't depend on what changed, so reusing them is safe.
                    render_cache: Mutex::new(self.carried_render.clone().unwrap_or_default()),
                })
            })
            .as_ref()
            .map_err(to_io_err)
    }

    /// Reset the lazily-built template/render cache after an incremental `regenerate`. Template
    /// data (font hash, `src`, contexts) is rebuilt fresh, but rendered strings that provably do
    /// not depend on changed template inputs are carried forward into the next cache.
    pub(crate) fn reset_render_cache(&mut self, names_unchanged: bool, codepoints_unchanged: bool) {
        let carried = self
            .cached
            .get()
            .and_then(|cached| cached.as_ref().ok())
            .map(|cached| {
                let css_deps = cached.shared.css_template_dependencies;
                let css_no_urls_unchanged =
                    css_deps.can_reuse_css_no_urls(names_unchanged, codepoints_unchanged);
                let css_with_urls_unchanged =
                    css_deps.can_reuse_css_with_urls(names_unchanged, codepoints_unchanged);
                let html_no_urls_unchanged = cached.html_template_dependencies.can_reuse_html(
                    names_unchanged,
                    codepoints_unchanged,
                    css_no_urls_unchanged,
                );
                let html_with_urls_unchanged = cached.html_template_dependencies.can_reuse_html(
                    names_unchanged,
                    codepoints_unchanged,
                    css_with_urls_unchanged,
                );

                let rc = cached.render_cache.lock().unwrap();
                RenderCache {
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
                }
            });
        self.cached = OnceLock::new();
        self.css_context = None;
        self.html_context = None;
        self.carried_render = carried;
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
                let result = crate::util::render_with_field_swap(
                    &mut html_ctx,
                    "styles",
                    serde_json::Value::String(styles),
                    |ctx| {
                        registry
                            .render_with_context("html", ctx)
                            .map_err(crate::util::to_io_err)
                    },
                )?;
                rc.html_last_urls = Some(urls.clone());
                rc.html_last_result = Some(result.clone());
                Ok(result)
            }
        }
    }
}

// NAPI getters and methods
#[cfg(feature = "napi")]
#[napi]
impl GenerateWebfontsResult {
    /// EOT font bytes, or `null` if EOT was not in `types`.
    #[napi(getter)]
    pub fn eot(&self) -> Option<Uint8Array> {
        self.fonts
            .eot_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// SVG font XML string, or `null` if SVG was not in `types`.
    #[napi(getter)]
    pub fn svg(&self) -> Option<String> {
        self.fonts.svg_font.as_ref().map(|v| v.as_ref().clone())
    }

    /// TTF font bytes, or `null` if TTF was not in `types`.
    #[napi(getter)]
    pub fn ttf(&self) -> Option<Uint8Array> {
        self.fonts
            .ttf_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// WOFF2 font bytes, or `null` if WOFF2 was not in `types`.
    #[napi(getter)]
    pub fn woff2(&self) -> Option<Uint8Array> {
        self.fonts
            .woff2_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// WOFF font bytes, or `null` if WOFF was not in `types`.
    #[napi(getter)]
    pub fn woff(&self) -> Option<Uint8Array> {
        self.fonts
            .woff_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// Render the CSS string for this result. Pass `urls` to override the
    /// default font URLs in the `@font-face src:` descriptor (only the keys
    /// you supply are overridden). The result is cached per `urls` value, so
    /// repeated calls with the same input are cheap.
    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_css(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        self.generate_css_pure(urls)
            .map_err(crate::util::to_napi_err)
    }

    /// Render the HTML preview string for this result. Pass `urls` to
    /// override font URLs in the embedded stylesheet (only the keys you
    /// supply are overridden). The result is cached per `urls` value.
    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_html(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        self.generate_html_pure(urls)
            .map_err(crate::util::to_napi_err)
    }

    /// Rebuild the font after a batch of file changes, reusing cached glyph geometry for files
    /// whose contents are unchanged. Requires the font to have been generated with
    /// `incremental: true`. `files` is the complete file set after the changes, in the order a
    /// fresh build would use (e.g. the glob result) — the rebuilt glyphs are ordered to match it,
    /// so the output bytes are identical to a fresh `generateWebfonts` of that set. `changes`
    /// describes the affected files: added/changed files are re-read from disk; any file absent
    /// from `files` is dropped. Omit `changes` to re-read/hash every current file and infer
    /// added/changed/removed paths from `files`. Every requested format is refreshed in memory,
    /// and — like `generateWebfonts` — when the result was built with `writeFiles` enabled the
    /// refreshed fonts are written to disk too, while CSS/HTML companion files are skipped if their
    /// rendered bytes are unchanged since the last write.
    #[napi(js_name = "regenerate")]
    pub fn regenerate_from_js(
        &mut self,
        files: Vec<String>,
        changes: Option<Vec<GlyphChangeEntry>>,
    ) -> napi::Result<()> {
        let Some(changes) = changes else {
            return self
                .regenerate_all(&files)
                .map_err(crate::util::to_napi_err);
        };
        let changes = changes
            .into_iter()
            .map(|entry| {
                let change = match entry.change_type.as_str() {
                    "added" => GlyphChange::Added { name: entry.name },
                    "changed" => GlyphChange::Changed { name: entry.name },
                    "removed" => GlyphChange::Removed,
                    other => {
                        return Err(napi::Error::from_reason(format!(
                            "Unknown changeType '{other}'; expected 'added', 'changed', or 'removed'."
                        )));
                    }
                };
                Ok((entry.path, change))
            })
            .collect::<napi::Result<Vec<_>>>()?;
        self.regenerate(&files, &changes)
            .map_err(crate::util::to_napi_err)
    }
}

#[cfg(feature = "napi")]
fn parse_native_urls(urls: HashMap<String, String>) -> napi::Result<HashMap<FontType, String>> {
    urls.into_iter()
        .filter_map(|(font_type, url)| {
            let font_type = match font_type.as_str() {
                "svg" => Some(FontType::Svg),
                "ttf" => Some(FontType::Ttf),
                "eot" => Some(FontType::Eot),
                "woff" => Some(FontType::Woff),
                "woff2" => Some(FontType::Woff2),
                _ => None,
            }?;

            Some(Ok((font_type, url)))
        })
        .collect::<napi::Result<HashMap<FontType, String>>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{finalize_generate_webfonts_options, resolve_generate_webfonts_options};

    fn build_result(template: Option<&str>) -> GenerateWebfontsResult {
        let fixture = crate::test_helpers::webfont_fixture("add.svg");

        let mut css_template = None;
        let cleanup_dir;
        if let Some(content) = template {
            let tmp = std::env::temp_dir().join(format!(
                "render-cache-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&tmp).unwrap();
            let path = tmp.join("template.hbs");
            std::fs::write(&path, content).unwrap();
            css_template = Some(path.to_string_lossy().into_owned());
            cleanup_dir = Some(tmp);
        } else {
            cleanup_dir = None;
        }

        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template,
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            dest: "artifacts".to_owned(),
            files: vec![fixture],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            start_codepoint: Some(0xE001),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let mut resolved = resolve_generate_webfonts_options(options).unwrap();
        let source_files: Vec<LoadedSvgFile> = resolved
            .files
            .iter()
            .map(|path| LoadedSvgFile {
                contents: std::fs::read_to_string(path).unwrap(),
                glyph_name: std::path::Path::new(path)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
                path: path.clone(),
            })
            .collect();
        finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();

        let result = GenerateWebfontsResult {
            cached: std::sync::OnceLock::new(),
            carried_render: None,
            css_context: None,
            fonts: FontOutputs::default(),
            glyph_cache: None,
            html_context: None,
            options: resolved,
            source_files,
            ttf_cache: None,
            written_outputs: Default::default(),
        };

        if let Some(dir) = cleanup_dir {
            // Don't clean up yet -- template file needed for lazy compilation
            std::mem::forget(dir);
        }

        result
    }

    #[test]
    fn generate_css_returns_cached_result_on_repeated_calls_without_urls() {
        let result = build_result(None);

        let first = result.generate_css_pure(None).unwrap();
        let second = result.generate_css_pure(None).unwrap();

        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn generate_css_returns_cached_result_on_repeated_calls_with_same_urls() {
        let result = build_result(None);
        let urls = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);

        let first = result.generate_css_pure(Some(urls.clone())).unwrap();
        let second = result.generate_css_pure(Some(urls)).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("/a.svg"));
    }

    #[test]
    fn generate_css_returns_different_result_for_different_urls() {
        let result = build_result(None);
        let urls_a = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);
        let urls_b = HashMap::from([(FontType::Svg, "/b.svg".to_owned())]);

        let result_a = result.generate_css_pure(Some(urls_a)).unwrap();
        let result_b = result.generate_css_pure(Some(urls_b)).unwrap();

        assert_ne!(result_a, result_b);
        assert!(result_a.contains("/a.svg"));
        assert!(result_b.contains("/b.svg"));
    }

    #[test]
    fn generate_css_cache_updates_when_urls_change() {
        let result = build_result(None);
        let urls_a = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);
        let urls_b = HashMap::from([(FontType::Svg, "/b.svg".to_owned())]);

        let first_a = result.generate_css_pure(Some(urls_a.clone())).unwrap();
        let first_b = result.generate_css_pure(Some(urls_b)).unwrap();
        let second_a = result.generate_css_pure(Some(urls_a)).unwrap();

        assert_eq!(
            first_a, second_a,
            "returning to original urls should produce same result"
        );
        assert_ne!(first_a, first_b);
    }

    #[test]
    fn generate_css_cache_works_with_custom_template() {
        let result = build_result(Some("@font-face { src: {{{src}}}; }"));
        let urls = HashMap::from([(FontType::Svg, "/cached.svg".to_owned())]);

        let first = result.generate_css_pure(Some(urls.clone())).unwrap();
        let second = result.generate_css_pure(Some(urls)).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("/cached.svg"));
    }

    #[test]
    fn generate_css_no_urls_and_with_urls_are_independent_caches() {
        let result = build_result(None);
        let urls = HashMap::from([(FontType::Svg, "/custom.svg".to_owned())]);

        let no_urls = result.generate_css_pure(None).unwrap();
        let with_urls = result.generate_css_pure(Some(urls)).unwrap();
        let no_urls_again = result.generate_css_pure(None).unwrap();

        assert_eq!(
            no_urls, no_urls_again,
            "no-urls cache should survive a with-urls call"
        );
        assert_ne!(no_urls, with_urls);
    }

    #[test]
    fn generate_css_with_urls_returns_no_urls_result_when_template_does_not_use_src() {
        let result = build_result(Some(".icon { font-family: {{fontName}}; }"));
        let urls = HashMap::from([(FontType::Svg, "/should-not-appear.svg".to_owned())]);

        let no_urls = result.generate_css_pure(None).unwrap();
        let with_urls = result.generate_css_pure(Some(urls)).unwrap();

        assert_eq!(
            no_urls, with_urls,
            "template without {{src}} should ignore urls"
        );
        assert!(!with_urls.contains("/should-not-appear.svg"));
        assert!(
            with_urls.contains("iconfont"),
            "should still render the template"
        );
    }

    #[test]
    fn generate_html_with_urls_returns_no_urls_result_when_css_template_does_not_use_src() {
        let result = build_result(Some(".icon { font-family: {{fontName}}; }"));
        let urls = HashMap::from([(FontType::Svg, "/should-not-appear.svg".to_owned())]);

        let no_urls = result.generate_html_pure(None).unwrap();
        let with_urls = result.generate_html_pure(Some(urls)).unwrap();

        assert_eq!(
            no_urls, with_urls,
            "CSS template without {{src}} means HTML is also unaffected by urls"
        );
    }

    #[test]
    fn generate_css_without_urls_produces_valid_css_using_css_fonts_url() {
        let result = build_result(None);

        let css = result.generate_css_pure(None).unwrap();

        assert!(
            css.contains("@font-face"),
            "should contain @font-face declaration"
        );
        assert!(css.contains("font-family:"), "should contain font-family");
        assert!(
            css.contains("iconfont.svg?"),
            "should use font name in URL with hash"
        );
        assert!(
            css.contains("format(\"svg\")"),
            "should contain format declaration"
        );
        assert!(
            css.contains("content:"),
            "should contain codepoint content rules"
        );
    }

    #[test]
    fn generate_css_with_urls_replaces_default_urls_in_src() {
        let result = build_result(None);
        let urls = HashMap::from([(FontType::Svg, "/cdn/icons.svg".to_owned())]);

        let css = result.generate_css_pure(Some(urls)).unwrap();

        assert!(
            css.contains("/cdn/icons.svg"),
            "custom URL should appear in output"
        );
        assert!(
            !css.contains("iconfont.svg?"),
            "default hash-based URL should not appear"
        );
        assert!(
            css.contains("format(\"svg\")"),
            "format should still be present"
        );
    }

    #[test]
    fn generate_html_without_urls_produces_valid_html() {
        let result = build_result(None);

        let html = result.generate_html_pure(None).unwrap();

        assert!(
            html.contains("<!DOCTYPE html>"),
            "should be a full HTML document"
        );
        assert!(html.contains("iconfont"), "should contain font name");
        assert!(html.contains("icon-add"), "should contain icon class name");
    }

    #[test]
    fn generate_html_with_urls_embeds_css_using_custom_urls() {
        let result = build_result(None);
        let urls = HashMap::from([(FontType::Svg, "/cdn/icons.svg".to_owned())]);

        let html = result.generate_html_pure(Some(urls)).unwrap();

        assert!(
            html.contains("/cdn/icons.svg"),
            "custom URL should appear in embedded CSS"
        );
        assert!(
            html.contains("icon-add"),
            "should still contain icon class name"
        );
    }

    #[test]
    fn generate_html_cache_returns_same_result_for_same_urls() {
        let result = build_result(None);
        let urls = HashMap::from([(FontType::Svg, "/cached.svg".to_owned())]);

        let first = result.generate_html_pure(Some(urls.clone())).unwrap();
        let second = result.generate_html_pure(Some(urls)).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("/cached.svg"));
    }

    #[test]
    fn generate_html_cache_returns_different_result_for_different_urls() {
        let result = build_result(None);
        let urls_a = HashMap::from([(FontType::Svg, "/a.svg".to_owned())]);
        let urls_b = HashMap::from([(FontType::Svg, "/b.svg".to_owned())]);

        let result_a = result.generate_html_pure(Some(urls_a)).unwrap();
        let result_b = result.generate_html_pure(Some(urls_b)).unwrap();

        assert_ne!(result_a, result_b);
        assert!(result_a.contains("/a.svg"));
        assert!(result_b.contains("/b.svg"));
    }

    /// Build a result with multiple font types (svg + woff2) for testing partial URL overrides.
    fn build_multi_type_result() -> GenerateWebfontsResult {
        let fixture = crate::test_helpers::webfont_fixture("add.svg");
        let options = GenerateWebfontsOptions {
            css: Some(true),
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            dest: "artifacts".to_owned(),
            files: vec![fixture],
            html: Some(true),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Woff2, FontType::Svg]),
            start_codepoint: Some(0xE001),
            types: Some(vec![FontType::Svg, FontType::Woff2]),
            ..Default::default()
        };

        let mut resolved = resolve_generate_webfonts_options(options).unwrap();
        let source_files: Vec<LoadedSvgFile> = resolved
            .files
            .iter()
            .map(|path| LoadedSvgFile {
                contents: std::fs::read_to_string(path).unwrap(),
                glyph_name: std::path::Path::new(path)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
                path: path.clone(),
            })
            .collect();
        finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();

        GenerateWebfontsResult {
            cached: std::sync::OnceLock::new(),
            carried_render: None,
            css_context: None,
            fonts: FontOutputs::default(),
            glyph_cache: None,
            html_context: None,
            options: resolved,
            source_files,
            ttf_cache: None,
            written_outputs: Default::default(),
        }
    }

    #[test]
    fn generate_css_partial_urls_uses_empty_string_for_missing_types() {
        let result = build_multi_type_result();
        // Override only woff2, leave svg un-provided -- matches upstream behavior
        let urls = HashMap::from([(FontType::Woff2, "/cdn/font.woff2".to_owned())]);

        let css = result.generate_css_pure(Some(urls)).unwrap();

        assert!(
            css.contains("/cdn/font.woff2"),
            "overridden URL should appear"
        );
        assert!(
            !css.contains("iconfont.svg?"),
            "non-overridden type should not have default hash-based URL"
        );
        assert!(
            css.contains("url(\"#iconfont\")"),
            "non-overridden SVG type should produce empty base URL (upstream compat)"
        );
    }

    #[test]
    fn generate_html_partial_urls_uses_empty_string_for_missing_types() {
        let result = build_multi_type_result();
        let urls = HashMap::from([(FontType::Woff2, "/cdn/font.woff2".to_owned())]);

        let html = result.generate_html_pure(Some(urls)).unwrap();

        assert!(
            html.contains("/cdn/font.woff2"),
            "overridden URL should appear in HTML"
        );
        assert!(
            !html.contains("iconfont.svg?"),
            "non-overridden type should not have default hash-based URL in HTML"
        );
    }
}
