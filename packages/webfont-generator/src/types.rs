use std::collections::HashMap;

#[cfg(feature = "napi")]
use napi_derive::napi;
use serde_json::{Map, Value};

/// What happened to a file, for [`crate::GenerateWebfontsResult::regenerate`]. `name` is the
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

/// One named SVG design in a multi-variant icon family.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct FontVariant {
    /// User-facing variant name, used to derive CSS modifier classes and an encoded filename
    /// component.
    pub name: String,
    /// SVG files that belong to this variant.
    pub files: Vec<String>,
    /// Optional explicit CSS weight coordinate in the range 1 through 1000. Automatic weights
    /// resolve outward from the default in steps of 100, or evenly when an interval is crowded.
    pub weight: Option<u16>,
    /// Whether this is the family's default variant. Exactly one variant must set this to `true`.
    pub default: Option<bool>,
}

/// Family-wide behavior when a logical glyph is absent from a variant.
#[cfg_attr(feature = "napi", napi(string_enum = "lowercase"))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MissingGlyphBehavior {
    /// Use an empty outline while retaining the logical glyph's advance.
    Blank,
    /// Reject generation and report every missing variant/glyph pair.
    Error,
    /// Reuse the outline from the named fallback variant.
    Fallback,
}

/// Family-wide missing-glyph policy for multi-variant generation.
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct MissingGlyphOptions {
    /// Missing-glyph behavior. Variant mode defaults to [`MissingGlyphBehavior::Blank`] when this
    /// object is omitted.
    pub behavior: MissingGlyphBehavior,
    /// Fallback variant name. Required only when `behavior` is `fallback`.
    pub variant: Option<String>,
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

/// Top-level options controlling webfont generation. `dest` and exactly one source, ordinary
/// `files` or future `variants`, are required. Variant input is validated and resolved before
/// returning an unsupported-operation error; every other field has a sensible default.
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
    /// Paths to the SVG files to include in an ordinary font. Must be empty when `variants` is set.
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
    /// Family-wide missing-glyph policy for multi-variant generation. Invalid without `variants`.
    pub missing_glyphs: Option<MissingGlyphOptions>,
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
    /// Prefix for generated variant modifier classes. Defaults to `icon--` in variant mode and is
    /// invalid without `variants`.
    pub variant_class_prefix: Option<String>,
    /// Ordered named SVG designs for one logical icon family. Variant generation is not yet
    /// available; valid input resolves metadata, loads and renames SVGs, joins logical glyphs, and
    /// assigns shared codepoints before returning an unsupported-operation error.
    pub variants: Option<Vec<FontVariant>>,
    /// Whether to write generated files to disk. Set to `false` for
    /// in-memory usage. Defaults to `true`.
    pub write_files: Option<bool>,
}
