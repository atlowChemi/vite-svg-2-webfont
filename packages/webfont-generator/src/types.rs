use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "napi")]
use napi::bindgen_prelude::Uint8Array;
#[cfg(feature = "napi")]
use napi_derive::napi;
use serde_json::{Map, Value};

use crate::input::LoadedSvgFile;
use crate::pipeline::TtfGlyphCache;
use crate::rendering::{CachedTemplateData, CarriedRenderCache};
use crate::svg::types::GlyphCache;

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

/// Rendered bytes for each requested output format. Held by [`GenerateWebfontsResult`] and
/// produced by the generator's format pipeline; grouping them lets an incremental regenerate
/// refresh every format in a single assignment.
#[derive(Clone, Default)]
pub(crate) struct FontOutputs {
    pub(crate) svg_font: Option<Arc<String>>,
    pub(crate) ttf_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff2_font: Option<Arc<Vec<u8>>>,
    pub(crate) eot_font: Option<Arc<Vec<u8>>>,
}

pub(crate) struct RegenerationState {
    pub(crate) caches_dirty: bool,
    pub(crate) glyph_cache: GlyphCache,
    pub(crate) ttf_cache: Option<TtfGlyphCache>,
    pub(crate) written_outputs: HashMap<String, [u8; 16]>,
}

pub(crate) struct RegenerationStateLease {
    slot: Arc<Mutex<Option<RegenerationState>>>,
    state: Option<RegenerationState>,
    keep_caches: bool,
}

/// Failure from asynchronous regeneration. Ordinary regeneration errors retain the input result
/// so callers can recover it with [`RegenerateError::into_result`] and retry.
pub struct RegenerateError {
    result: Option<Box<GenerateWebfontsResult>>,
    source: std::io::Error,
}

impl RegenerateError {
    pub(crate) fn new(result: Option<GenerateWebfontsResult>, source: std::io::Error) -> Self {
        Self {
            result: result.map(Box::new),
            source,
        }
    }

    /// Return the result that was consumed by the failed operation. This is `None` only if the
    /// blocking task was cancelled by the runtime before it could return the result.
    pub fn into_result(self) -> Option<GenerateWebfontsResult> {
        self.result.map(|result| *result)
    }

    /// Return both the recoverable result and the underlying I/O error.
    pub fn into_parts(self) -> (Option<GenerateWebfontsResult>, std::io::Error) {
        (self.result.map(|result| *result), self.source)
    }
}

impl std::fmt::Debug for RegenerateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegenerateError")
            .field("recoverable", &self.result.is_some())
            .field("source", &self.source)
            .finish()
    }
}

impl std::fmt::Display for RegenerateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for RegenerateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl RegenerationStateLease {
    pub(crate) fn state_mut(&mut self) -> &mut RegenerationState {
        self.state.as_mut().unwrap()
    }

    pub(crate) fn commit(mut self) {
        self.keep_caches = true;
    }
}

impl Drop for RegenerationStateLease {
    fn drop(&mut self) {
        let mut state = self.state.take().unwrap();
        if !self.keep_caches && state.caches_dirty {
            state.glyph_cache = GlyphCache::default();
            if state.ttf_cache.is_some() {
                state.ttf_cache = Some(TtfGlyphCache::default());
            }
        }
        state.caches_dirty = false;
        *self.slot.lock().unwrap() = Some(state);
    }
}

/// Result of a successful `generateWebfonts` call. Exposes the generated
/// font bytes (or `null` for formats that were not requested) and methods to
/// render the CSS and HTML preview.
#[cfg_attr(feature = "napi", napi)]
pub struct GenerateWebfontsResult {
    pub(crate) cached: OnceLock<Result<CachedTemplateData, String>>,
    /// Render-cache entries carried across an incremental `regenerate` to seed the rebuilt
    /// [`CachedTemplateData`], so CSS/HTML that
    /// doesn't depend on what changed isn't re-rendered. `None` for a normal build.
    pub(crate) carried_render: Option<CarriedRenderCache>,
    pub(crate) css_context: Option<Map<String, Value>>,
    pub(crate) fonts: FontOutputs,
    pub(crate) html_context: Option<Map<String, Value>>,
    pub(crate) options: Arc<ResolvedGenerateWebfontsOptions>,
    pub(crate) regeneration_state: Arc<Mutex<Option<RegenerationState>>>,
    pub(crate) source_files: Arc<Vec<LoadedSvgFile>>,
}

// Pure Rust getters (always available)
impl GenerateWebfontsResult {
    #[cfg(any(feature = "napi", feature = "bench"))]
    pub(crate) fn snapshot_for_regeneration(&self, state: RegenerationState) -> Self {
        Self {
            cached: OnceLock::new(),
            carried_render: self.render_cache_source(),
            css_context: self.css_context.clone(),
            fonts: self.fonts.clone(),
            html_context: self.html_context.clone(),
            options: self.options.clone(),
            regeneration_state: Arc::new(Mutex::new(Some(state))),
            source_files: self.source_files.clone(),
        }
    }

    pub(crate) fn take_regeneration_state(&self) -> std::io::Result<RegenerationState> {
        if !self.options.incremental {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "regenerate requires the font to be generated with `incremental` enabled.",
            ));
        }
        self.regeneration_state
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "This result is already regenerating or has been replaced.",
                )
            })
    }

    pub(crate) fn take_regeneration_state_lease(&self) -> std::io::Result<RegenerationStateLease> {
        Ok(RegenerationStateLease {
            slot: Arc::clone(&self.regeneration_state),
            state: Some(self.take_regeneration_state()?),
            keep_caches: false,
        })
    }

    #[cfg(feature = "bench")]
    pub(crate) fn restore_regeneration_state(&self, state: RegenerationState) {
        *self.regeneration_state.lock().unwrap() = Some(state);
    }

    pub(crate) fn seed_written_outputs(&self, written_outputs: HashMap<String, [u8; 16]>) {
        if let Some(state) = self.regeneration_state.lock().unwrap().as_mut() {
            state.written_outputs = written_outputs;
        }
    }

    #[doc(hidden)]
    #[cfg(feature = "bench")]
    pub fn regenerate_owned_for_bench(
        &self,
        files: &[String],
        changes: &[(String, GlyphChange)],
    ) -> std::io::Result<Self> {
        let state = self.take_regeneration_state()?;
        let mut replacement = self.snapshot_for_regeneration(state);
        match replacement.regenerate(files, changes) {
            Ok(()) => Ok(replacement),
            Err(error) => {
                if let Ok(state) = replacement.take_regeneration_state() {
                    self.restore_regeneration_state(state);
                }
                Err(error)
            }
        }
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
        self.generate_css_pure(urls).map_err(to_napi_err)
    }

    /// Render the HTML preview string for this result. Pass `urls` to
    /// override font URLs in the embedded stylesheet (only the keys you
    /// supply are overridden). The result is cached per `urls` value.
    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_html(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        self.generate_html_pure(urls).map_err(to_napi_err)
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
        let changes = parse_glyph_changes(changes)?;
        match changes {
            Some(changes) => self.regenerate(&files, &changes),
            None => self.regenerate_all(&files),
        }
        .map_err(to_napi_err)
    }

    /// Rebuild off the Node.js event loop and resolve with a replacement result. The receiver
    /// remains readable and unchanged while regeneration runs and after failure. Assign the
    /// resolved result before starting another regeneration. Overlapping calls on the same result
    /// lineage are rejected, and disk writes remain non-transactional.
    #[napi(js_name = "regenerateAsync")]
    pub async fn regenerate_async_from_js(
        &self,
        files: Vec<String>,
        changes: Option<Vec<GlyphChangeEntry>>,
    ) -> napi::Result<GenerateWebfontsResult> {
        let changes = parse_glyph_changes(changes)?;
        let state = self.take_regeneration_state().map_err(to_napi_err)?;
        let original_state = Arc::clone(&self.regeneration_state);
        let mut replacement = self.snapshot_for_regeneration(state);
        tokio::task::spawn_blocking(move || -> std::io::Result<GenerateWebfontsResult> {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match changes {
                Some(changes) => replacement.regenerate(&files, &changes),
                None => replacement.regenerate_all(&files),
            }));
            match result {
                Ok(Ok(())) => Ok(replacement),
                Ok(Err(error)) => {
                    if let Ok(state) = replacement.take_regeneration_state() {
                        *original_state.lock().unwrap() = Some(state);
                    }
                    Err(error)
                }
                Err(payload) => {
                    if let Ok(state) = replacement.take_regeneration_state() {
                        *original_state.lock().unwrap() = Some(state);
                    }
                    std::panic::resume_unwind(payload)
                }
            }
        })
        .await
        .map_err(|error| {
            napi::Error::from_reason(format!("Native webfont regeneration task failed: {error}"))
        })?
        .map_err(to_napi_err)
    }
}

#[cfg(feature = "napi")]
fn to_napi_err(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::new(napi::Status::GenericFailure, error.to_string())
}

#[cfg(feature = "napi")]
fn parse_glyph_changes(
    changes: Option<Vec<GlyphChangeEntry>>,
) -> napi::Result<Option<Vec<(String, GlyphChange)>>> {
    changes
        .map(|changes| {
            changes
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
                .collect()
        })
        .transpose()
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
