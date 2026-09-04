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
mod result;
mod sfnt;
mod svg;
#[cfg(test)]
mod test_helpers;
mod types;

#[cfg(feature = "napi")]
use napi::Status;
#[cfg(feature = "napi")]
use napi::threadsafe_function::ThreadsafeFunction;
#[cfg(feature = "napi")]
use napi_derive::napi;
#[cfg(feature = "napi")]
use std::sync::Mutex;

use input::{
    ResolvedGenerateWebfontsOptions, build_variant_family_sources,
    finalize_generate_webfonts_options, load_svg_files, load_variant_svg_files,
    resolve_generate_webfonts_options, resolve_missing_glyphs, validate_generate_webfonts_options,
};
#[cfg(feature = "napi")]
use input::{load_svg_files_napi, load_variant_svg_files_napi};
use output::write_generate_webfonts_result;
use pipeline::generate_webfonts_sync;
#[cfg(feature = "napi")]
use rendering::{
    CachedTemplateData, SharedTemplateData, apply_context_function, build_css_context,
    build_html_context, build_html_registry_and_dependencies,
};
#[cfg(feature = "napi")]
use result::to_napi_err;
pub use result::{GenerateWebfontsResult, RegenerateError};
pub use types::{
    CssContext, FontType, FontVariant, FormatOptions, GenerateWebfontsOptions, GlyphChange,
    GlyphChangeEntry, HtmlContext, MissingGlyphBehavior, MissingGlyphOptions, SvgFormatOptions,
    TtfFormatOptions, Woff2FormatOptions, WoffFormatOptions,
};

fn prepare_variant_family(
    options: &mut ResolvedGenerateWebfontsOptions,
    source_files: Vec<Vec<input::LoadedSvgFile>>,
) -> std::io::Result<svg::types::PreparedVariantFamily> {
    let (mut family, codepoints) = build_variant_family_sources(
        source_files,
        &options.explicit_codepoints,
        options.start_codepoint,
    )?;
    let variants = options
        .variants
        .as_ref()
        .expect("variant source preparation requires resolved variants");
    let variant_names = variants
        .variants
        .iter()
        .map(|variant| variant.name.as_str())
        .collect::<Vec<_>>();
    let fallback_index = options.missing_glyphs.variant.as_deref().map(|name| {
        variant_names
            .iter()
            .position(|variant_name| *variant_name == name)
            .expect("validated fallback must name a resolved variant")
    });
    resolve_missing_glyphs(
        &mut family,
        options.missing_glyphs.behavior,
        fallback_index,
        &variant_names,
    )?;
    options.codepoints = codepoints;
    svg::prepare_variant_svg_family(&svg::svg_options_from_options(options), &family)
}

fn unavailable_variant_generation() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Multi-variant generation is not available yet; this release resolves variant sources, missing-glyph behavior, and shared geometry.",
    )
}

#[cfg(feature = "napi")]
fn variant_preparation_join_error(error: tokio::task::JoinError) -> napi::Error {
    napi::Error::new(
        Status::GenericFailure,
        format!("Native variant preparation task failed: {error}"),
    )
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

#[cfg(all(test, feature = "napi"))]
#[unsafe(no_mangle)]
extern "C" fn napi_release_threadsafe_function(
    _: napi::sys::napi_threadsafe_function,
    _: napi::sys::napi_threadsafe_function_release_mode,
) -> napi::sys::napi_status {
    0
}

#[cfg(all(test, feature = "napi"))]
#[tokio::test]
async fn napi_variant_preparation_reports_panicking_worker() {
    let join_error = tokio::task::spawn_blocking(|| panic!("variant preparation panic"))
        .await
        .unwrap_err();
    let error = variant_preparation_join_error(join_error);

    assert!(
        error
            .reason
            .starts_with("Native variant preparation task failed:")
    );
}

#[cfg(all(test, feature = "napi"))]
#[tokio::test]
async fn napi_variant_generation_prepares_sources_before_returning_unsupported() {
    let path = test_helpers::webfont_fixture("add.svg");
    let variants = vec![
        FontVariant {
            name: "small".to_owned(),
            files: vec![path.clone()],
            weight: Some(300),
            default: Some(true),
        },
        FontVariant {
            name: "large".to_owned(),
            files: vec![path.clone()],
            weight: Some(700),
            default: None,
        },
    ];
    let options = GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec![],
        types: Some(vec![FontType::Woff2]),
        variants: Some(variants.clone()),
        ..Default::default()
    };

    let error = generate_webfonts(options, None, None, None)
        .await
        .err()
        .expect("variant generation should remain unavailable");
    assert!(error.reason.contains("not available yet"));

    let mut duplicate = variants;
    duplicate[0].files.push(path);
    let error = generate_webfonts(
        GenerateWebfontsOptions {
            dest: "artifacts".to_owned(),
            files: vec![],
            types: Some(vec![FontType::Woff2]),
            variants: Some(duplicate),
            ..Default::default()
        },
        None,
        None,
        None,
    )
    .await
    .err()
    .expect("duplicate names within one variant should fail");
    assert!(error.reason.contains("must be unique"));
}

#[cfg(all(test, feature = "napi"))]
#[tokio::test]
async fn napi_generation_keeps_the_ordinary_source_path() {
    let result = generate_webfonts(
        GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec![test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            types: Some(vec![FontType::Svg]),
            write_files: Some(false),
            ..Default::default()
        },
        None,
        None,
        None,
    )
    .await
    .expect("ordinary NAPI generation should succeed");

    assert!(result.svg_string().is_some());
}

/// Generate a webfont from a set of SVG files.
///
/// Loads the SVGs listed in `options.files`, builds the configured
/// `options.types` formats, optionally writes them (along with the CSS and
/// HTML preview) to `options.dest`, and returns a `GenerateWebfontsResult`
/// holding the font bytes and template-rendering methods.
///
/// Multi-variant input is resolved, loaded, renamed, joined into logical glyphs, assigned shared
/// codepoints and metrics, and processed according to its missing-glyph policy before returning an
/// unsupported-operation error.
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
    if options.variants.is_some() {
        let mut resolved_options = resolve_generate_webfonts_options(options)?;
        let variant_paths = resolved_options
            .variants
            .as_ref()
            .expect("validated variant options must resolve variants")
            .variants
            .iter()
            .map(|variant| variant.files.clone())
            .collect::<Vec<_>>();
        let source_files = load_variant_svg_files_napi(&variant_paths, rename.as_ref()).await?;
        let preparation = tokio::task::spawn_blocking(move || {
            prepare_variant_family(&mut resolved_options, source_files)
        });
        let preparation = preparation.await;
        let _family = preparation.map_err(variant_preparation_join_error)??;
        return Err(unavailable_variant_generation().into());
    }
    let source_files = load_svg_files_napi(&options.files, rename.as_ref(), true).await?;
    let mut resolved_options = resolve_generate_webfonts_options(options)?;
    finalize_generate_webfonts_options(&mut resolved_options, &source_files)?;

    let mut result =
        tokio::task::spawn_blocking(move || generate_webfonts_sync(resolved_options, source_files))
            .await
            .map_err(|error| {
                napi::Error::new(
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
/// This is the pure Rust async entry point. Requires a tokio runtime. Multi-variant input is
/// resolved, loaded, renamed, joined into logical glyphs, assigned shared codepoints and metrics,
/// and processed according to its missing-glyph policy before returning an unsupported-operation
/// error.
pub async fn generate(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult> {
    validate_generate_webfonts_options(&options)?;
    if options.variants.is_some() {
        let mut resolved_options = resolve_generate_webfonts_options(options)?;
        let variant_paths = resolved_options
            .variants
            .as_ref()
            .expect("validated variant options must resolve variants")
            .variants
            .iter()
            .map(|variant| variant.files.clone())
            .collect::<Vec<_>>();
        let source_files = load_variant_svg_files(&variant_paths, rename.as_deref()).await?;
        let preparation = tokio::task::spawn_blocking(move || {
            prepare_variant_family(&mut resolved_options, source_files)
        });
        let _family = preparation.await.map_err(std::io::Error::other)??;
        return Err(unavailable_variant_generation());
    }
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

/// Synchronous version of [`generate`]. Spawns a tokio runtime internally and has the same source
/// preparation behavior for multi-variant input.
pub fn generate_sync(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult> {
    tokio::runtime::Runtime::new()?.block_on(generate(options, rename))
}
