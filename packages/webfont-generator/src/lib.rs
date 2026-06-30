//! # webfont-generator
//!
//! Generate webfonts (SVG, TTF, EOT, WOFF, WOFF2) from SVG icon files.
//!
//! ## Library usage
//!
//! ```rust,no_run
//! use webfont_generator::{GenerateWebfontsOptions, FontType};
//!
//! // Async API (requires a tokio runtime)
//! # async fn example() -> std::io::Result<()> {
//! let options = GenerateWebfontsOptions {
//!     dest: "output".to_owned(),
//!     files: vec!["icons/add.svg".to_owned(), "icons/remove.svg".to_owned()],
//!     font_name: Some("my-icons".to_owned()),
//!     types: Some(vec![FontType::Woff2, FontType::Woff]),
//!     ..Default::default()
//! };
//!
//! let result = webfont_generator::generate(options, None).await?;
//! if let Some(woff2) = result.woff2_bytes() {
//!     println!("Generated WOFF2: {} bytes", woff2.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ```rust,no_run
//! use webfont_generator::{GenerateWebfontsOptions, FontType};
//!
//! // Synchronous API
//! let options = GenerateWebfontsOptions {
//!     dest: "output".to_owned(),
//!     files: vec!["icons/add.svg".to_owned()],
//!     write_files: Some(false),
//!     ..Default::default()
//! };
//!
//! let result = webfont_generator::generate_sync(options, None).unwrap();
//! ```
//!
//! ## CLI
//!
//! Install the CLI binary with:
//!
//! ```sh
//! cargo install webfont-generator --features cli
//! ```
//!
//! Then run:
//!
//! ```sh
//! webfont-generator --dest ./dist/fonts ./icons/
//! ```
//!
//! ## Feature flags
//!
//! - **`cli`**: Builds the `webfont-generator` CLI binary (adds `clap` dependency).
//!   Not enabled by default — use `cargo install webfont-generator --features cli`.
//! - **`napi`**: Enables Node.js NAPI bindings for use as a native addon.

mod eot;
mod incremental;
mod sfnt;
mod svg;
mod templates;
#[cfg(test)]
mod test_helpers;
mod ttf;
mod types;
mod util;
mod woff;
mod write;

#[cfg(feature = "napi")]
use napi::threadsafe_function::ThreadsafeFunction;
#[cfg(feature = "napi")]
use napi::{Error as NapiError, Status};
#[cfg(feature = "napi")]
use napi_derive::napi;
use rayon::join;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;
#[cfg(feature = "napi")]
use std::sync::Mutex;
use tokio::task::JoinSet;

use svg::types::{GlyphCache, PreparedSvgFont, SvgOptions};
use svg::{
    build_svg_font, prepare_svg_font, prepare_svg_font_incremental, svg_options_from_options,
};
#[cfg(feature = "napi")]
use templates::{
    SharedTemplateData, apply_context_function, build_css_context, build_html_context,
    build_html_registry_and_dependencies,
};
use ttf::TtfGlyphCache;
#[cfg(feature = "napi")]
use util::to_napi_err;
use write::write_generate_webfonts_result;

pub use types::{
    CssContext, FontType, FormatOptions, GenerateWebfontsOptions, GenerateWebfontsResult,
    GlyphChange, GlyphChangeEntry, HtmlContext, SvgFormatOptions, TtfFormatOptions,
    Woff2FormatOptions, WoffFormatOptions,
};
use types::{
    DEFAULT_FONT_ORDER, FontOutputs, LoadedSvgFile, ResolvedGenerateWebfontsOptions,
    resolved_font_types,
};

#[cfg(feature = "bench")]
pub mod bench_support {
    use std::io;

    use super::{
        GenerateWebfontsOptions, GenerateWebfontsResult, GlyphCache, LoadedSvgFile,
        PreparedSvgFont, build_font_outputs, finalize_generate_webfonts_options, prepare_svg_font,
        prepare_svg_font_incremental, resolve_generate_webfonts_options, svg_options_from_options,
    };
    use crate::svg::types::ParsedGlyph;
    use crate::svg::{finalize_glyphs, parse_glyphs};

    /// Source fixture used by Rust benchmarks without exposing generator internals.
    #[derive(Clone)]
    pub struct BenchSvgSource {
        pub path: String,
        pub glyph_name: String,
        pub contents: String,
    }

    /// Opaque parsed-glyph cache used by incremental SVG prepare benchmarks.
    #[derive(Clone, Default)]
    pub struct BenchGlyphCache(GlyphCache);

    /// Opaque parsed glyph set used to isolate parse and finalize stages.
    #[derive(Clone)]
    pub struct BenchParsedGlyphs(Vec<ParsedGlyph>);

    /// Opaque prepared SVG font used to isolate font-output generation stages.
    #[derive(Clone)]
    pub struct BenchPreparedSvgFont(PreparedSvgFont);

    fn load_sources(sources: &[BenchSvgSource]) -> Vec<LoadedSvgFile> {
        sources
            .iter()
            .map(|source| LoadedSvgFile {
                contents: source.contents.clone(),
                glyph_name: source.glyph_name.clone(),
                path: source.path.clone(),
            })
            .collect()
    }

    fn resolve(
        options: GenerateWebfontsOptions,
        sources: &[LoadedSvgFile],
    ) -> io::Result<super::ResolvedGenerateWebfontsOptions> {
        let mut options = resolve_generate_webfonts_options(options)?;
        finalize_generate_webfonts_options(&mut options, sources)?;
        Ok(options)
    }

    /// Run the SVG parse+process preparation path and return the number of prepared glyphs.
    pub fn prepare_svg_full(
        options: GenerateWebfontsOptions,
        sources: &[BenchSvgSource],
    ) -> io::Result<usize> {
        let sources = load_sources(sources);
        let options = resolve(options, &sources)?;
        let svg_options = svg_options_from_options(&options);
        let prepared = prepare_svg_font(&svg_options, &sources)?;
        Ok(prepared.processed_glyphs.len())
    }

    /// Parse SVG glyph geometry without running set-wide finalization/processing.
    pub fn parse_svg_only(
        options: GenerateWebfontsOptions,
        sources: &[BenchSvgSource],
    ) -> io::Result<BenchParsedGlyphs> {
        let sources = load_sources(sources);
        let options = resolve(options, &sources)?;
        let svg_options = svg_options_from_options(&options);
        parse_glyphs(&svg_options, &sources).map(BenchParsedGlyphs)
    }

    /// Run set-wide SVG finalization/processing from already parsed glyph geometry.
    pub fn finalize_svg_only(
        options: GenerateWebfontsOptions,
        sources: &[BenchSvgSource],
        parsed: BenchParsedGlyphs,
    ) -> io::Result<BenchPreparedSvgFont> {
        let sources = load_sources(sources);
        let options = resolve(options, &sources)?;
        let svg_options = svg_options_from_options(&options);
        finalize_glyphs(&svg_options, parsed.0).map(BenchPreparedSvgFont)
    }

    /// Build requested font outputs from an already prepared SVG font and return total output bytes.
    pub fn build_outputs_only(
        options: GenerateWebfontsOptions,
        sources: &[BenchSvgSource],
        prepared: &BenchPreparedSvgFont,
    ) -> io::Result<usize> {
        let sources = load_sources(sources);
        let options = resolve(options, &sources)?;
        let svg_options = svg_options_from_options(&options);
        let fonts = build_font_outputs(&options, &svg_options, &prepared.0, None)?;
        Ok(fonts.svg_font.as_ref().map_or(0, |v| v.len())
            + fonts.ttf_font.as_ref().map_or(0, |v| v.len())
            + fonts.woff_font.as_ref().map_or(0, |v| v.len())
            + fonts.woff2_font.as_ref().map_or(0, |v| v.len())
            + fonts.eot_font.as_ref().map_or(0, |v| v.len()))
    }

    /// Clear retained WOFF1 payloads so benchmarks can compare warm vs cold compression cache.
    pub fn clear_woff1_payload_cache(result: &mut GenerateWebfontsResult) {
        if let Some(cache) = result.ttf_cache.as_mut() {
            cache.clear_woff1_payloads();
        }
    }

    /// Run the incremental SVG preparation path and return the number of prepared glyphs.
    pub fn prepare_svg_incremental(
        options: GenerateWebfontsOptions,
        sources: &[BenchSvgSource],
        cache: &mut BenchGlyphCache,
    ) -> io::Result<usize> {
        let sources = load_sources(sources);
        let options = resolve(options, &sources)?;
        let svg_options = svg_options_from_options(&options);
        let prepared = prepare_svg_font_incremental(&svg_options, &sources, &mut cache.0)?;
        Ok(prepared.processed_glyphs.len())
    }
}

#[cfg(all(test, feature = "napi"))]
#[unsafe(no_mangle)]
extern "C" fn napi_call_threadsafe_function(
    _: napi::sys::napi_threadsafe_function,
    _: *mut std::ffi::c_void,
    _: napi::sys::napi_threadsafe_function_call_mode,
) -> napi::sys::napi_status {
    0
}

/// Generate a webfont from a set of SVG files.
///
/// Loads the SVGs listed in `options.files`, builds the configured
/// `options.types` formats, optionally writes them (along with the CSS and
/// HTML preview) to `options.dest`, and returns a `GenerateWebfontsResult`
/// holding the font bytes and template-rendering methods.
///
/// Optional callbacks:
/// - `rename(path)` — derive a custom glyph name from each SVG file path.
/// - `cssContext(ctx)` — mutate the Handlebars context before CSS rendering;
///   return the (possibly mutated) context.
/// - `htmlContext(ctx)` — same, but for the HTML preview.
#[cfg(feature = "napi")]
#[napi]
#[allow(clippy::type_complexity)] // NAPI proc macro requires the verbose ThreadsafeFunction type
pub async fn generate_webfonts(
    options: GenerateWebfontsOptions,
    rename: Option<ThreadsafeFunction<String, String, String, Status, false>>,
    css_context: Option<
        ThreadsafeFunction<
            serde_json::Map<String, serde_json::Value>,
            serde_json::Map<String, serde_json::Value>,
            serde_json::Map<String, serde_json::Value>,
            Status,
            false,
        >,
    >,
    html_context: Option<
        ThreadsafeFunction<
            serde_json::Map<String, serde_json::Value>,
            serde_json::Map<String, serde_json::Value>,
            serde_json::Map<String, serde_json::Value>,
            Status,
            false,
        >,
    >,
) -> napi::Result<GenerateWebfontsResult> {
    validate_generate_webfonts_options(&options)?;
    let source_files = load_svg_files_napi(&options.files, rename.as_ref()).await?;
    let mut resolved_options = resolve_generate_webfonts_options(options)?;
    finalize_generate_webfonts_options(&mut resolved_options, &source_files)?;

    let mut result =
        tokio::task::spawn_blocking(move || generate_webfonts_sync(resolved_options, source_files))
            .await
            .map_err(|error| {
                NapiError::new(
                    Status::GenericFailure,
                    format!("Native webfont generation task failed: {error}"),
                )
            })??;

    // Pre-compute mutated contexts via ThreadsafeFunction (async-safe).
    // When callbacks are present, we build SharedTemplateData here and seed the
    // OnceLock cache so it isn't re-created in get_cached() / writeFiles.
    if css_context.is_some() || html_context.is_some() {
        let shared =
            SharedTemplateData::new(&result.options, &result.source_files).map_err(to_napi_err)?;

        let mut css_ctx = build_css_context(&result.options, &shared);
        if css_context.is_some() {
            css_ctx = apply_context_function(css_ctx, css_context.as_ref())
                .await
                .map_err(to_napi_err)?;
            result.css_context = Some(css_ctx.clone());
        }

        let mut html_ctx = if result.options.html || html_context.is_some() {
            build_html_context(&result.options, &shared, &result.source_files, None)
                .map_err(to_napi_err)?
        } else {
            serde_json::Map::new()
        };
        if html_context.is_some() {
            html_ctx = apply_context_function(html_ctx, html_context.as_ref())
                .await
                .map_err(to_napi_err)?;
            result.html_context = Some(html_ctx.clone());
        }

        // Seed the OnceLock -- avoids re-creating SharedTemplateData in get_cached()
        let (html_registry, html_template_dependencies) =
            build_html_registry_and_dependencies(&result.options).map_err(to_napi_err)?;
        let css_hbs_context = handlebars::Context::wraps(&css_ctx).map_err(to_napi_err)?;
        let html_hbs_context = handlebars::Context::wraps(&html_ctx).map_err(to_napi_err)?;
        let _ = result.cached.set(Ok(types::CachedTemplateData {
            shared,
            css_context: css_ctx,
            css_hbs_context: Mutex::new(css_hbs_context),
            html_context: html_ctx,
            html_hbs_context: Mutex::new(html_hbs_context),
            html_template_dependencies,
            html_registry,
            render_cache: Mutex::new(Default::default()),
        }));
    }

    if result.options.write_files
        && let Some(written) = write_generate_webfonts_result(&result).await?
    {
        // Only incremental results can call `regenerate`, so only they need write-skip state.
        result.written_outputs = written;
    }

    Ok(result)
}

/// A glyph rename function that maps file stems to custom glyph names.
pub type RenameFn = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Generate webfonts from SVG files.
///
/// This is the pure Rust async entry point. Requires a tokio runtime.
pub async fn generate(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult> {
    validate_generate_webfonts_options(&options)?;
    let source_files = load_svg_files(&options.files, rename.as_deref()).await?;
    let mut resolved_options = resolve_generate_webfonts_options(options)?;
    finalize_generate_webfonts_options(&mut resolved_options, &source_files)?;

    let mut result =
        tokio::task::spawn_blocking(move || generate_webfonts_sync(resolved_options, source_files))
            .await
            .map_err(std::io::Error::other)??;

    if result.options.write_files
        && let Some(written) = write_generate_webfonts_result(&result).await?
    {
        // Only incremental results can call `regenerate`, so only they need write-skip state.
        result.written_outputs = written;
    }

    Ok(result)
}

/// Synchronous version of [`generate`]. Spawns a tokio runtime internally.
pub fn generate_sync(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult> {
    tokio::runtime::Runtime::new()?.block_on(generate(options, rename))
}

fn validate_generate_webfonts_options(options: &GenerateWebfontsOptions) -> std::io::Result<()> {
    if options.dest.is_empty() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "\"options.dest\" is empty.".to_owned(),
        ));
    }

    if options.files.is_empty() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "\"options.files\" is empty.".to_owned(),
        ));
    }

    if options.css.unwrap_or(true)
        && let Some(ref path) = options.css_template
        && !Path::new(path).exists()
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("\"options.cssTemplate\" file not found: {path}"),
        ));
    }

    if options.html.unwrap_or(false)
        && let Some(ref path) = options.html_template
        && !Path::new(path).exists()
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("\"options.htmlTemplate\" file not found: {path}"),
        ));
    }

    if let Some(quality) = options
        .format_options
        .as_ref()
        .and_then(|value| value.woff2.as_ref())
        .and_then(|value| value.compression_quality)
        && quality > 11
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "\"options.formatOptions.woff2.compressionQuality\" must be between 0 and 11, got {quality}."
            ),
        ));
    }

    Ok(())
}

pub(crate) fn resolve_generate_webfonts_options(
    options: GenerateWebfontsOptions,
) -> std::io::Result<ResolvedGenerateWebfontsOptions> {
    let types = resolved_font_types(&options);
    validate_font_type_order(&options, &types)?;
    let order = resolve_font_type_order(&options, &types);
    let css = options.css.unwrap_or(true);
    let html = options.html.unwrap_or(false);
    let font_name = options.font_name.unwrap_or_else(|| "iconfont".to_owned());
    let css_dest = options
        .css_dest
        .unwrap_or_else(|| default_output_dest(&options.dest, &font_name, "css"));
    let html_dest = options
        .html_dest
        .unwrap_or_else(|| default_output_dest(&options.dest, &font_name, "html"));
    let write_files = options.write_files.unwrap_or(true);
    let explicit_codepoints: std::collections::BTreeMap<String, u32> =
        options.codepoints.unwrap_or_default().into_iter().collect();

    let svg_format = options
        .format_options
        .as_ref()
        .and_then(|fo| fo.svg.as_ref());
    let center_vertically = svg_format
        .and_then(|s| s.center_vertically)
        .or(options.center_vertically);
    let optimize_output = svg_format
        .and_then(|s| s.optimize_output)
        .or(options.optimize_output);
    let preserve_aspect_ratio = svg_format
        .and_then(|s| s.preserve_aspect_ratio)
        .or(options.preserve_aspect_ratio);

    Ok(ResolvedGenerateWebfontsOptions {
        ascent: options.ascent,
        center_horizontally: options.center_horizontally,
        center_vertically,
        css,
        css_dest,
        css_template: match options.css_template {
            Some(ref t) if t.is_empty() => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.cssTemplate\" must not be empty.".to_owned(),
                ));
            }
            other => other,
        },
        codepoints: explicit_codepoints.clone(),
        explicit_codepoints,
        css_fonts_url: options.css_fonts_url,
        descent: options.descent,
        dest: options.dest,
        files: options.files,
        fixed_width: options.fixed_width,
        format_options: options.format_options,
        html,
        html_dest,
        html_template: match options.html_template {
            Some(ref t) if t.is_empty() => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "\"options.htmlTemplate\" must not be empty.".to_owned(),
                ));
            }
            other => other,
        },
        incremental: options.incremental.unwrap_or(false),
        font_height: options.font_height,
        font_name,
        font_style: options.font_style,
        font_weight: options.font_weight,
        ligature: options.ligature.unwrap_or(true),
        normalize: options.normalize.unwrap_or(true),
        order,
        optimize_output,
        preserve_aspect_ratio,
        round: options.round,
        start_codepoint: options.start_codepoint.unwrap_or(0xF101),
        template_options: options.template_options,
        types,
        write_files,
    })
}

pub(crate) fn finalize_generate_webfonts_options(
    options: &mut ResolvedGenerateWebfontsOptions,
    source_files: &[LoadedSvgFile],
) -> std::io::Result<()> {
    options.codepoints = resolve_codepoints(
        source_files,
        &options.explicit_codepoints,
        options.start_codepoint,
    )?;

    Ok(())
}

fn resolve_font_type_order(options: &GenerateWebfontsOptions, types: &[FontType]) -> Vec<FontType> {
    match &options.order {
        Some(order) => order.clone(),
        None => DEFAULT_FONT_ORDER
            .iter()
            .copied()
            .filter(|font_type| types.contains(font_type))
            .collect(),
    }
}

fn default_output_dest(dest: &str, font_name: &str, extension: &str) -> String {
    Path::new(dest)
        .join(format!("{font_name}.{extension}"))
        .to_string_lossy()
        .into_owned()
}

fn generate_webfonts_sync(
    options: ResolvedGenerateWebfontsOptions,
    source_files: Vec<LoadedSvgFile>,
) -> std::io::Result<GenerateWebfontsResult> {
    let svg_options = svg_options_from_options(&options);
    // When incremental, retain the parsed-glyph cache so a later `regenerate` can reuse the
    // glyphs whose source didn't change. Otherwise the geometry is dropped as soon as the font
    // is built, so one-shot builds carry no extra memory.
    let (prepared, glyph_cache, mut ttf_cache) = if options.incremental {
        let mut cache = GlyphCache::default();
        let prepared = prepare_svg_font_incremental(&svg_options, &source_files, &mut cache)?;
        (prepared, Some(cache), Some(TtfGlyphCache::default()))
    } else {
        (prepare_svg_font(&svg_options, &source_files)?, None, None)
    };
    let fonts = build_font_outputs(&options, &svg_options, &prepared, ttf_cache.as_mut())?;

    Ok(GenerateWebfontsResult {
        cached: std::sync::OnceLock::new(),
        carried_render: None,
        css_context: None,
        fonts,
        glyph_cache,
        html_context: None,
        options,
        source_files,
        ttf_cache,
        written_outputs: std::collections::HashMap::new(),
    })
}

/// Build every requested output format from an already-prepared glyph set.
fn build_font_outputs(
    options: &ResolvedGenerateWebfontsOptions,
    svg_options: &SvgOptions<'_>,
    prepared: &PreparedSvgFont,
    mut ttf_cache: Option<&mut TtfGlyphCache>,
) -> std::io::Result<FontOutputs> {
    let wants_svg = options.types.contains(&FontType::Svg);
    let wants_ttf = options.types.contains(&FontType::Ttf);
    let wants_woff = options.types.contains(&FontType::Woff);
    let wants_woff2 = options.types.contains(&FontType::Woff2);
    let wants_eot = options.types.contains(&FontType::Eot);

    let (svg_font, ttf_tables) = join(
        || -> std::io::Result<Option<String>> {
            if wants_svg {
                Ok(Some(build_svg_font(svg_options, prepared)))
            } else {
                Ok(None)
            }
        },
        || -> std::io::Result<Option<sfnt::SerializedFontTables>> {
            if wants_ttf || wants_woff || wants_woff2 || wants_eot {
                let ttf_options = ttf::ttf_options_from_options(options);
                match ttf_cache.as_deref_mut() {
                    Some(cache) => ttf::generate_ttf_font_from_glyphs_cached(
                        ttf_options,
                        &prepared.processed_glyphs,
                        cache,
                    )
                    .map(Some),
                    None => {
                        ttf::generate_ttf_font_from_glyphs(ttf_options, &prepared.processed_glyphs)
                            .map(Some)
                    }
                }
            } else {
                Ok(None)
            }
        },
    );

    let svg_font = svg_font?.map(Arc::new);
    let ttf_tables = ttf_tables?;

    let (ttf_font, woff_font, woff2_font, eot_font) = if let Some(ttf_tables) = ttf_tables {
        let woff_metadata = options
            .format_options
            .as_ref()
            .and_then(|value| value.woff.as_ref())
            .and_then(|value| value.metadata.as_deref());
        let woff2_quality = options
            .format_options
            .as_ref()
            .and_then(|value| value.woff2.as_ref())
            .and_then(|value| value.compression_quality)
            .unwrap_or(11);

        let ttf_tables = Arc::new(ttf_tables);
        let raw_ttf = (wants_ttf || wants_woff2).then(|| ttf_tables.ttf_arc());
        let ttf_font = wants_ttf.then(|| Arc::clone(raw_ttf.as_ref().unwrap()));
        let (woff_font, (woff2_font, eot_font)) = join(
            || -> std::io::Result<Option<Vec<u8>>> {
                if wants_woff {
                    match ttf_cache {
                        Some(cache) => {
                            woff::tables_to_woff1_cached(&ttf_tables, woff_metadata, cache)
                        }
                        None => woff::tables_to_woff1(&ttf_tables, woff_metadata),
                    }
                    .map(Some)
                } else {
                    Ok(None)
                }
            },
            || {
                join(
                    || -> std::io::Result<Option<Vec<u8>>> {
                        if wants_woff2 {
                            woff::ttf_to_woff2(raw_ttf.as_ref().unwrap(), woff2_quality).map(Some)
                        } else {
                            Ok(None)
                        }
                    },
                    || -> std::io::Result<Option<Vec<u8>>> {
                        if wants_eot {
                            eot::tables_to_eot(&ttf_tables).map(Some)
                        } else {
                            Ok(None)
                        }
                    },
                )
            },
        );

        (
            ttf_font,
            woff_font?.map(Arc::new),
            woff2_font?.map(Arc::new),
            eot_font?.map(Arc::new),
        )
    } else {
        (None, None, None, None)
    };

    Ok(FontOutputs {
        svg_font,
        ttf_font,
        woff_font,
        woff2_font,
        eot_font,
    })
}

fn validate_font_type_order(
    options: &GenerateWebfontsOptions,
    requested_types: &[FontType],
) -> std::io::Result<()> {
    if let Some(order) = &options.order
        && let Some(invalid_type) = order
            .iter()
            .copied()
            .find(|font_type| !requested_types.contains(font_type))
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Invalid font type order: '{}' is not present in 'types'.",
                invalid_type.as_extension()
            ),
        ));
    }

    Ok(())
}

/// Load SVG file contents in parallel, preserving the original order.
async fn load_svg_contents(paths: &[String]) -> std::io::Result<Vec<(String, String)>> {
    let mut tasks = JoinSet::new();

    for (index, path) in paths.iter().cloned().enumerate() {
        tasks.spawn(async move {
            tokio::fs::read_to_string(&path)
                .await
                .map(|contents| (index, (path, contents)))
        });
    }

    let mut results = Vec::with_capacity(paths.len());
    while let Some(result) = tasks.join_next().await {
        let (index, pair) = result
            .map_err(|error| std::io::Error::other(format!("SVG loading task failed: {error}")))?
            .map_err(|error| {
                std::io::Error::other(format!("Failed to read source SVG file: {error}"))
            })?;
        results.push((index, pair));
    }

    results.sort_by_key(|(index, _)| *index);
    Ok(results.into_iter().map(|(_, pair)| pair).collect())
}

/// Load SVG files and resolve glyph names using an optional sync rename function.
async fn load_svg_files(
    paths: &[String],
    rename: Option<&(dyn Fn(&str) -> String + Send + Sync)>,
) -> std::io::Result<Vec<LoadedSvgFile>> {
    let raw = load_svg_contents(paths).await?;
    let source_files: Vec<LoadedSvgFile> = raw
        .into_iter()
        .map(|(path, contents)| {
            let glyph_name = util::glyph_name_from_path(&path, rename)?;
            Ok(LoadedSvgFile {
                contents,
                glyph_name,
                path,
            })
        })
        .collect::<std::io::Result<_>>()?;

    validate_glyph_names(&source_files)?;
    Ok(source_files)
}

/// NAPI version: resolve glyph names via async ThreadsafeFunction callback.
#[cfg(feature = "napi")]
async fn load_svg_files_napi(
    paths: &[String],
    rename: Option<
        &napi::threadsafe_function::ThreadsafeFunction<String, String, String, Status, false>,
    >,
) -> napi::Result<Vec<LoadedSvgFile>> {
    let raw = load_svg_contents(paths).await.map_err(to_napi_err)?;
    let mut source_files = Vec::with_capacity(raw.len());

    for (path, contents) in raw {
        let glyph_name = if let Some(rename) = rename {
            rename.call_async(path.clone()).await?
        } else {
            util::default_glyph_name_from_path(&path).map_err(to_napi_err)?
        };
        source_files.push(LoadedSvgFile {
            contents,
            glyph_name,
            path,
        });
    }

    validate_glyph_names(&source_files).map_err(to_napi_err)?;
    Ok(source_files)
}

pub(crate) fn validate_glyph_names(source_files: &[LoadedSvgFile]) -> std::io::Result<()> {
    let mut seen_names = HashSet::with_capacity(source_files.len());

    for source_file in source_files {
        if !seen_names.insert(source_file.glyph_name.clone()) {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "The glyph name \"{}\" must be unique.",
                    source_file.glyph_name
                ),
            ));
        }
    }

    Ok(())
}

// Re-export resolve_codepoints for use in finalize_generate_webfonts_options
use util::resolve_codepoints;

#[cfg(test)]
mod tests {
    use super::{
        resolve_generate_webfonts_options, resolved_font_types, validate_font_type_order,
        validate_generate_webfonts_options, woff,
    };
    use crate::{
        FontType, FormatOptions, GenerateWebfontsOptions, Woff2FormatOptions,
        ttf::generate_ttf_font_bytes,
    };

    #[test]
    fn generates_woff2_font_with_expected_header() {
        let ttf_result = generate_ttf_font_bytes(GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec![format!(
                "{}/../vite-svg-2-webfont/src/fixtures/webfont-test/svg/add.svg",
                env!("CARGO_MANIFEST_DIR")
            )],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            ..Default::default()
        })
        .expect("expected ttf generation to succeed");

        let result = woff::ttf_to_woff2(&ttf_result, 10).expect("woff2 generation should succeed");

        assert_eq!(&result[..4], b"wOF2");
    }

    #[test]
    fn rejects_order_entries_that_are_not_present_in_types() {
        let options = GenerateWebfontsOptions {
            dest: "artifacts".to_owned(),
            files: vec![],
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg, FontType::Woff]),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let error = validate_font_type_order(&options, &resolved_font_types(&options)).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            error
                .to_string()
                .contains("Invalid font type order: 'woff' is not present in 'types'.")
        );
    }

    #[test]
    fn rejects_an_empty_dest() {
        let options = GenerateWebfontsOptions {
            dest: String::new(),
            files: vec!["icon.svg".to_owned()],
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let error = validate_generate_webfonts_options(&options).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("\"options.dest\" is empty."));
    }

    #[test]
    fn rejects_empty_files() {
        let options = GenerateWebfontsOptions {
            dest: "artifacts".to_owned(),
            files: vec![],
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let error = validate_generate_webfonts_options(&options).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("\"options.files\" is empty."));
    }

    fn options_with_woff2_quality(quality: u8) -> GenerateWebfontsOptions {
        GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            font_name: Some("iconfont".to_owned()),
            format_options: Some(FormatOptions {
                woff2: Some(Woff2FormatOptions {
                    compression_quality: Some(quality),
                }),
                ..Default::default()
            }),
            html: Some(false),
            ligature: Some(false),
            types: Some(vec![FontType::Woff2]),
            ..Default::default()
        }
    }

    #[test]
    fn rejects_woff2_compression_quality_above_11() {
        let error =
            validate_generate_webfonts_options(&options_with_woff2_quality(12)).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains(
            "\"options.formatOptions.woff2.compressionQuality\" must be between 0 and 11, got 12."
        ));
    }

    #[test]
    fn accepts_woff2_compression_quality_of_11() {
        validate_generate_webfonts_options(&options_with_woff2_quality(11))
            .expect("compression quality 11 is the upper bound and must be accepted");
    }

    #[test]
    fn rejects_empty_css_template() {
        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some(String::new()),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let error = resolve_generate_webfonts_options(options)
            .err()
            .expect("expected empty css template to fail");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            error
                .to_string()
                .contains("\"options.cssTemplate\" must not be empty.")
        );
    }

    #[test]
    fn rejects_empty_html_template() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            html: Some(true),
            html_template: Some(String::new()),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let error = resolve_generate_webfonts_options(options)
            .err()
            .expect("expected empty html template to fail");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            error
                .to_string()
                .contains("\"options.htmlTemplate\" must not be empty.")
        );
    }

    #[test]
    fn resolves_write_defaults_from_dest_and_font_name() {
        let options = GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let resolved = resolve_generate_webfonts_options(options)
            .expect("expected defaults to resolve successfully");

        assert!(resolved.write_files);
        assert_eq!(resolved.css_dest, "artifacts/iconfont.css");
        assert_eq!(resolved.html_dest, "artifacts/iconfont.html");
    }

    #[test]
    fn rejects_nonexistent_css_template_when_css_is_true() {
        let error = validate_generate_webfonts_options(&GenerateWebfontsOptions {
            css: Some(true),
            css_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            html: Some(false),
            ..Default::default()
        })
        .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("cssTemplate"));
    }

    #[test]
    fn allows_nonexistent_css_template_when_css_is_false() {
        validate_generate_webfonts_options(&GenerateWebfontsOptions {
            css: Some(false),
            css_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            html: Some(false),
            ..Default::default()
        })
        .expect("should allow nonexistent css template when css is false");
    }

    #[test]
    fn rejects_nonexistent_html_template_when_html_is_true() {
        let error = validate_generate_webfonts_options(&GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            html: Some(true),
            html_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
            ..Default::default()
        })
        .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("htmlTemplate"));
    }

    #[test]
    fn allows_nonexistent_html_template_when_html_is_false() {
        validate_generate_webfonts_options(&GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec!["icon.svg".to_owned()],
            html: Some(false),
            html_template: Some("/tmp/__nonexistent_template__.hbs".to_owned()),
            ..Default::default()
        })
        .expect("should allow nonexistent html template when html is false");
    }
}
