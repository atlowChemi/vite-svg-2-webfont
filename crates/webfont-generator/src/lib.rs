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
//!
//! Node.js bindings are built by the separate, unpublished `webfont-generator-napi` crate.

#[cfg(feature = "bench")]
pub mod bench_support;
mod byte_helpers;
mod formats;
mod hooks;
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

pub use hooks::GenerationHooks;
use std::sync::Mutex;

use input::{
    ResolvedGenerateWebfontsOptions, build_variant_family_sources,
    finalize_generate_webfonts_options, resolve_generate_webfonts_options, resolve_missing_glyphs,
    validate_generate_webfonts_options,
};
use input::{load_svg_files_with_hooks, load_variant_svg_files_with_hooks};
use output::write_generate_webfonts_result;
use pipeline::{generate_variant_webfonts_sync, generate_webfonts_sync};
use rendering::{
    CachedTemplateData, SharedTemplateData, build_css_context, build_html_context,
    build_html_registry_and_dependencies,
};
pub use result::{GenerateWebfontsResult, RegenerateError};
pub use types::{
    CssContext, FontType, FontVariant, FormatOptions, GenerateWebfontsOptions, GlyphChange,
    GlyphChangeEntry, HtmlContext, MissingGlyphBehavior, MissingGlyphOptions, RegenerationFiles,
    SvgFormatOptions, TemplateVariant, TtfFormatOptions, VariantFileSet, Woff2FormatOptions,
    WoffFormatOptions,
};

type PreparedVariantInput = (
    svg::types::PreparedVariantFamily,
    Vec<input::LoadedSvgFile>,
    Option<svg::VariantGlyphCache>,
);

fn prepare_variant_family(
    options: &mut ResolvedGenerateWebfontsOptions,
    source_files: Vec<Vec<input::LoadedSvgFile>>,
) -> std::io::Result<PreparedVariantInput> {
    let mut cache = options.incremental.then(svg::VariantGlyphCache::default);
    let family = prepare_variant_family_cached(
        options,
        source_files.iter().map(Vec::as_slice).collect(),
        cache.as_mut(),
    )?;
    let sources = source_files.into_iter().flatten().collect();
    Ok((family, sources, cache))
}

fn prepare_variant_family_cached(
    options: &mut ResolvedGenerateWebfontsOptions,
    source_files: Vec<&[input::LoadedSvgFile]>,
    cache: Option<&mut svg::VariantGlyphCache>,
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
    let svg_options = svg::svg_options_from_options(options);
    let prepared = match cache {
        Some(cache) => svg::prepare_variant_svg_family_cached(&svg_options, &family, Some(cache))?,
        None => svg::prepare_variant_svg_family(&svg_options, &family)?,
    };
    Ok(prepared)
}

fn variant_preparation_join_error(error: tokio::task::JoinError) -> std::io::Error {
    std::io::Error::other(format!("Native variant preparation task failed: {error}"))
}

#[cfg(test)]
#[tokio::test]
async fn variant_preparation_reports_panicking_worker() {
    let join_error = tokio::task::spawn_blocking(|| panic!("variant preparation panic"))
        .await
        .unwrap_err();
    let error = variant_preparation_join_error(join_error);

    assert!(
        error
            .to_string()
            .starts_with("Native variant preparation task failed:")
    );
}

#[cfg(test)]
#[tokio::test]
async fn hooked_variant_generation_succeeds_after_preparing_sources() {
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
        write_files: Some(false),
        ..Default::default()
    };

    let result = generate_with_hooks(options, &())
        .await
        .expect("variant generation should succeed");
    assert!(result.woff2_bytes().is_some());
    assert!(result.eot_bytes().is_none());

    let mut duplicate = variants;
    duplicate[0].files.push(path);
    let error = generate_with_hooks(
        GenerateWebfontsOptions {
            dest: "artifacts".to_owned(),
            files: vec![],
            types: Some(vec![FontType::Woff2]),
            variants: Some(duplicate),
            ..Default::default()
        },
        &(),
    )
    .await
    .err()
    .expect("duplicate names within one variant should fail");
    assert!(error.to_string().contains("must be unique"));
}

#[cfg(test)]
#[tokio::test]
async fn hooked_generation_keeps_the_ordinary_source_path() {
    let result = generate_with_hooks(
        GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec![test_helpers::webfont_fixture("add.svg")],
            html: Some(false),
            types: Some(vec![FontType::Svg]),
            write_files: Some(false),
            ..Default::default()
        },
        &(),
    )
    .await
    .expect("ordinary generation with hooks should succeed");

    assert!(result.svg_string().is_some());
}

/// Generate a webfont from a set of SVG files.
///
/// Loads the SVGs listed in `options.files`, builds the configured
/// `options.types` formats, optionally writes them (along with the CSS and
/// HTML preview) to `options.dest`, and returns a `GenerateWebfontsResult`
/// holding the font bytes and template-rendering methods.
///
/// Multi-variant input generates one shared variable font per requested modern format.
///
/// Runtime-independent asynchronous callbacks, supplied through [`GenerationHooks`]:
/// - `rename(paths)` — derive custom glyph names for the batch of SVG file paths.
/// - `css_context(ctx)` — mutate the Handlebars context before CSS rendering;
///   return the (possibly mutated) context.
/// - `html_context(ctx)` — same, but for the HTML preview.
pub async fn generate_with_hooks<H: GenerationHooks>(
    options: GenerateWebfontsOptions,
    hooks: &H,
) -> Result<GenerateWebfontsResult, H::Error> {
    validate_generate_webfonts_options(&options)?;
    let mut result = if options.variants.is_some() {
        let mut resolved_options = resolve_generate_webfonts_options(options)?;
        let variant_paths = resolved_options
            .variants
            .as_ref()
            .expect("validated variant options must resolve variants")
            .variants
            .iter()
            .map(|variant| variant.files.clone())
            .collect::<Vec<_>>();
        let source_files = load_variant_svg_files_with_hooks(&variant_paths, hooks).await?;
        let generation = tokio::task::spawn_blocking(move || {
            let (family, source_files, cache) =
                prepare_variant_family(&mut resolved_options, source_files)?;
            generate_variant_webfonts_sync(resolved_options, source_files, family, cache)
        });
        generation.await.map_err(variant_preparation_join_error)??
    } else {
        let source_files = load_svg_files_with_hooks(&options.files, hooks, true).await?;
        let mut resolved_options = resolve_generate_webfonts_options(options)?;
        finalize_generate_webfonts_options(&mut resolved_options, &source_files)?;
        tokio::task::spawn_blocking(move || generate_webfonts_sync(resolved_options, source_files))
            .await
            .map_err(|error| {
                std::io::Error::other(format!("Native webfont generation task failed: {error}"))
            })??
    };

    // Pre-compute mutated contexts through runtime-independent async hooks.
    // When callbacks are present, we build SharedTemplateData here and seed the
    // OnceLock cache so it isn't re-created in get_cached() / writeFiles.
    if hooks.has_css_context() || hooks.has_html_context() {
        let shared = SharedTemplateData::new(&result.options, &result.source_files)?;

        let mut css_ctx = build_css_context(&result.options, &shared);
        if hooks.has_css_context() {
            css_ctx = hooks.css_context(css_ctx).await?;
            css_ctx.insert(
                "__webfontVariantMode".to_owned(),
                serde_json::Value::Bool(result.options.variants.is_some()),
            );
            result.css_context = Some(css_ctx.clone());
        }

        let mut html_ctx =
            if result.options.html || hooks.has_html_context() || result.options.variants.is_some()
            {
                if result.options.variants.is_some() && hooks.has_css_context() {
                    rendering::build_html_context_with_css(
                        &result.options,
                        &shared,
                        &result.source_files,
                        &css_ctx,
                    )?
                } else {
                    build_html_context(&result.options, &shared, &result.source_files, None)?
                }
            } else {
                serde_json::Map::new()
            };
        if hooks.has_html_context() {
            html_ctx = hooks.html_context(html_ctx).await?;
            result.html_context = Some(html_ctx.clone());
        }

        // Seed the OnceLock -- avoids re-creating SharedTemplateData in get_cached()
        let (html_registry, html_template_dependencies) =
            build_html_registry_and_dependencies(&result.options)?;
        let css_hbs_context =
            handlebars::Context::wraps(&css_ctx).map_err(std::io::Error::other)?;
        let html_hbs_context =
            handlebars::Context::wraps(&html_ctx).map_err(std::io::Error::other)?;
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
/// This is the pure Rust async entry point. Requires a tokio runtime. Multi-variant input generates
/// one shared variable font per requested modern format.
pub async fn generate(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult> {
    generate_with_hooks(options, &hooks::RenameHooks(rename.as_deref())).await
}

/// Synchronous version of [`generate`]. Spawns a tokio runtime internally and has the same source
/// preparation behavior for multi-variant input.
pub fn generate_sync(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult> {
    tokio::runtime::Runtime::new()?.block_on(generate(options, rename))
}
