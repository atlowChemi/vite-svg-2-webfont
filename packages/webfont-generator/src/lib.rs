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

#[cfg(feature = "bench")]
pub mod bench_support;
mod byte_helpers;
mod formats;
mod incremental;
mod input;
mod output;
mod pipeline;
mod rendering;
mod sfnt;
mod svg;
#[cfg(test)]
mod test_helpers;
mod types;
mod woff;

#[cfg(feature = "napi")]
use napi::threadsafe_function::ThreadsafeFunction;
#[cfg(feature = "napi")]
use napi::{Error as NapiError, Status};
#[cfg(feature = "napi")]
use napi_derive::napi;
#[cfg(feature = "napi")]
use std::sync::Mutex;

#[cfg(feature = "napi")]
use input::load_svg_files_napi;
use input::{
    finalize_generate_webfonts_options, load_svg_files, resolve_generate_webfonts_options,
    validate_generate_webfonts_options,
};
use output::write_generate_webfonts_result;
use pipeline::generate_webfonts_sync;
#[cfg(feature = "napi")]
use rendering::{
    CachedTemplateData, SharedTemplateData, apply_context_function, build_css_context,
    build_html_context, build_html_registry_and_dependencies,
};
pub use types::{
    CssContext, FontType, FormatOptions, GenerateWebfontsOptions, GenerateWebfontsResult,
    GlyphChange, GlyphChangeEntry, HtmlContext, RegenerateError, SvgFormatOptions,
    TtfFormatOptions, Woff2FormatOptions, WoffFormatOptions,
};

#[cfg(feature = "napi")]
fn to_napi_err(error: impl std::fmt::Display) -> NapiError {
    NapiError::new(Status::GenericFailure, error.to_string())
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
/// - `rename(paths)` — derive custom glyph names for the batch of SVG file paths.
/// - `cssContext(ctx)` — mutate the Handlebars context before CSS rendering;
///   return the (possibly mutated) context.
/// - `htmlContext(ctx)` — same, but for the HTML preview.
#[cfg(feature = "napi")]
#[napi]
#[allow(clippy::type_complexity)] // NAPI proc macro requires the verbose ThreadsafeFunction type
pub async fn generate_webfonts(
    options: GenerateWebfontsOptions,
    rename: Option<ThreadsafeFunction<Vec<String>, Vec<String>, Vec<String>, Status, false>>,
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
        let _ = result.cached.set(Ok(CachedTemplateData {
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
        result.seed_written_outputs(written);
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

    let result =
        tokio::task::spawn_blocking(move || generate_webfonts_sync(resolved_options, source_files))
            .await
            .map_err(std::io::Error::other)??;

    if result.options.write_files
        && let Some(written) = write_generate_webfonts_result(&result).await?
    {
        // Only incremental results can call `regenerate`, so only they need write-skip state.
        result.seed_written_outputs(written);
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
