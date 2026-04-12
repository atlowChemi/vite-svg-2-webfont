mod eot;
mod svg;
mod templates;
#[cfg(test)]
mod test_helpers;
mod ttf;
mod types;
mod util;
mod woff;

use napi::threadsafe_function::ThreadsafeFunction;
use napi::{Error, Status};
use napi_derive::napi;
use rayon::join;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task::JoinSet;

use svg::{build_svg_font, prepare_svg_font, svg_options_from_options};
use templates::{
    apply_context_function, build_css_context, build_html_context, build_html_registry,
    render_css_with_hbs_context, render_html_with_hbs_context, SharedTemplateData,
};
use util::{glyph_name_from_path, resolve_codepoints, to_napi_err};

use types::{
    resolved_font_types, LoadedSvgFile, ResolvedGenerateWebfontsOptions, DEFAULT_FONT_ORDER,
};
pub use types::{
    FontType, FormatOptions, GenerateWebfontsOptions, GenerateWebfontsResult, SvgFormatOptions,
    TtfFormatOptions, WoffFormatOptions,
};

#[cfg(test)]
#[unsafe(no_mangle)]
extern "C" fn napi_call_threadsafe_function(
    _: napi::sys::napi_threadsafe_function,
    _: *mut std::ffi::c_void,
    _: napi::sys::napi_threadsafe_function_call_mode,
) -> napi::sys::napi_status {
    0
}

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
    let source_files = load_svg_files(&options.files, rename.as_ref()).await?;
    let mut resolved_options = resolve_generate_webfonts_options(options)?;
    finalize_generate_webfonts_options(&mut resolved_options, &source_files)?;

    let mut result =
        tokio::task::spawn_blocking(move || generate_webfonts_sync(resolved_options, source_files))
            .await
            .map_err(|error| {
                Error::new(
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

        // Seed the OnceLock — avoids re-creating SharedTemplateData in get_cached()
        let html_registry = build_html_registry(&result.options).map_err(to_napi_err)?;
        let css_hbs_context = handlebars::Context::wraps(&css_ctx).map_err(to_napi_err)?;
        let html_hbs_context = handlebars::Context::wraps(&html_ctx).map_err(to_napi_err)?;
        let _ = result.cached.set(Ok(types::CachedTemplateData {
            shared,
            css_context: css_ctx,
            css_hbs_context: Mutex::new(css_hbs_context),
            html_context: html_ctx,
            html_hbs_context: Mutex::new(html_hbs_context),
            html_registry,
            render_cache: Mutex::new(Default::default()),
        }));
    }

    if result.options.write_files {
        write_generate_webfonts_result(&result).await?;
    }

    Ok(result)
}

fn validate_generate_webfonts_options(options: &GenerateWebfontsOptions) -> napi::Result<()> {
    if options.dest.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "\"options.dest\" is empty.".to_owned(),
        ));
    }

    if options.files.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "\"options.files\" is empty.".to_owned(),
        ));
    }

    if options.css.unwrap_or(true) {
        if let Some(ref path) = options.css_template {
            if !Path::new(path).exists() {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("\"options.cssTemplate\" file not found: {path}"),
                ));
            }
        }
    }

    if options.html.unwrap_or(false) {
        if let Some(ref path) = options.html_template {
            if !Path::new(path).exists() {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("\"options.htmlTemplate\" file not found: {path}"),
                ));
            }
        }
    }

    Ok(())
}

pub(crate) fn resolve_generate_webfonts_options(
    options: GenerateWebfontsOptions,
) -> napi::Result<ResolvedGenerateWebfontsOptions> {
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
                return Err(Error::new(
                    Status::InvalidArg,
                    "\"options.cssTemplate\" must not be empty.".to_owned(),
                ))
            }
            other => other,
        },
        codepoints: options.codepoints.unwrap_or_default().into_iter().collect(),
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
                return Err(Error::new(
                    Status::InvalidArg,
                    "\"options.htmlTemplate\" must not be empty.".to_owned(),
                ))
            }
            other => other,
        },
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
) -> napi::Result<()> {
    options.codepoints =
        resolve_codepoints(source_files, &options.codepoints, options.start_codepoint)
            .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;

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
) -> napi::Result<GenerateWebfontsResult> {
    let wants_svg = options.types.contains(&FontType::Svg);
    let wants_ttf = options.types.contains(&FontType::Ttf);
    let wants_woff = options.types.contains(&FontType::Woff);
    let wants_woff2 = options.types.contains(&FontType::Woff2);
    let wants_eot = options.types.contains(&FontType::Eot);

    let svg_options = svg_options_from_options(&options);
    let prepared = prepare_svg_font(&svg_options, &source_files).map_err(to_napi_err)?;

    let (svg_font, raw_ttf) = join(
        || -> napi::Result<Option<String>> {
            if wants_svg {
                Ok(Some(build_svg_font(&svg_options, &prepared)))
            } else {
                Ok(None)
            }
        },
        || -> napi::Result<Option<Vec<u8>>> {
            if wants_ttf || wants_woff || wants_woff2 || wants_eot {
                let ttf_options = ttf::ttf_options_from_options(&options);
                ttf::generate_ttf_font_bytes_from_glyphs(ttf_options, &prepared.processed_glyphs)
                    .map(Some)
                    .map_err(to_napi_err)
            } else {
                Ok(None)
            }
        },
    );

    let svg_font = svg_font?.map(Arc::new);
    let raw_ttf = raw_ttf?;

    let (ttf_font, woff_font, woff2_font, eot_font) = if let Some(raw_ttf) = raw_ttf {
        let raw_ttf = Arc::new(raw_ttf);
        let ttf_font = wants_ttf.then(|| Arc::clone(&raw_ttf));
        let woff_metadata = options
            .format_options
            .as_ref()
            .and_then(|value| value.woff.as_ref())
            .and_then(|value| value.metadata.as_deref());

        let (woff_font, (woff2_font, eot_font)) = join(
            || -> napi::Result<Option<Vec<u8>>> {
                if wants_woff {
                    woff::ttf_to_woff1(&raw_ttf, woff_metadata)
                        .map(Some)
                        .map_err(to_napi_err)
                } else {
                    Ok(None)
                }
            },
            || {
                join(
                    || -> napi::Result<Option<Vec<u8>>> {
                        if wants_woff2 {
                            woff::ttf_to_woff2(&raw_ttf).map(Some).map_err(to_napi_err)
                        } else {
                            Ok(None)
                        }
                    },
                    || -> napi::Result<Option<Vec<u8>>> {
                        if wants_eot {
                            eot::ttf_to_eot(&raw_ttf).map(Some).map_err(to_napi_err)
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

    Ok(GenerateWebfontsResult {
        cached: std::sync::OnceLock::new(),
        css_context: None,
        eot_font,
        html_context: None,
        options,
        source_files,
        svg_font,
        ttf_font,
        woff2_font,
        woff_font,
    })
}

async fn write_generate_webfonts_result(result: &GenerateWebfontsResult) -> napi::Result<()> {
    let mut tasks = JoinSet::new();
    let font_name = result.options.font_name.clone();
    let dest = result.options.dest.clone();

    if let Some(svg_font) = &result.svg_font {
        let path = default_output_dest(&dest, &font_name, "svg");
        let contents = Arc::clone(svg_font);
        tasks.spawn(async move { write_output_file(path, contents.as_bytes()).await });
    }

    if let Some(ttf_font) = &result.ttf_font {
        let path = default_output_dest(&dest, &font_name, "ttf");
        let contents = Arc::clone(ttf_font);
        tasks.spawn(async move { write_output_file(path, &*contents).await });
    }

    if let Some(woff_font) = &result.woff_font {
        let path = default_output_dest(&dest, &font_name, "woff");
        let contents = Arc::clone(woff_font);
        tasks.spawn(async move { write_output_file(path, &*contents).await });
    }

    if let Some(woff2_font) = &result.woff2_font {
        let path = default_output_dest(&dest, &font_name, "woff2");
        let contents = Arc::clone(woff2_font);
        tasks.spawn(async move { write_output_file(path, &*contents).await });
    }

    if let Some(eot_font) = &result.eot_font {
        let path = default_output_dest(&dest, &font_name, "eot");
        let contents = Arc::clone(eot_font);
        tasks.spawn(async move { write_output_file(path, &*contents).await });
    }

    // Only render CSS/HTML templates when those files need to be written.
    if result.options.css || result.options.html {
        let cached = result.get_cached()?;

        if result.options.css {
            let ctx = cached.css_hbs_context.lock().unwrap();
            let css = render_css_with_hbs_context(&cached.shared, &ctx, &cached.css_context)
                .map_err(to_napi_err)?;
            drop(ctx);
            let css_dest = result.options.css_dest.clone();
            let css = Arc::new(css);
            tasks.spawn(async move { write_output_file(css_dest, css.as_bytes()).await });
        }

        if result.options.html {
            let ctx = cached.html_hbs_context.lock().unwrap();
            let html = render_html_with_hbs_context(
                cached.html_registry.as_ref(),
                &ctx,
                &cached.html_context,
            )
            .map_err(to_napi_err)?;
            let html_dest = result.options.html_dest.clone();
            tasks.spawn(async move { write_output_file(html_dest, html.into_bytes()).await });
        }
    }

    while let Some(result) = tasks.join_next().await {
        result
            .map_err(|error| {
                Error::new(
                    Status::GenericFailure,
                    format!("Native write task failed: {error}"),
                )
            })?
            .map_err(to_napi_err)?;
    }

    Ok(())
}

async fn write_output_file(path: String, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    if let Some(parent) = Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(path, contents).await
}

fn validate_font_type_order(
    options: &GenerateWebfontsOptions,
    requested_types: &[FontType],
) -> napi::Result<()> {
    if let Some(order) = &options.order {
        if let Some(invalid_type) = order
            .iter()
            .copied()
            .find(|font_type| !requested_types.contains(font_type))
        {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "Invalid font type order: '{}' is not present in 'types'.",
                    invalid_type.as_extension()
                ),
            ));
        }
    }

    Ok(())
}

async fn load_svg_files(
    paths: &[String],
    rename: Option<
        &napi::threadsafe_function::ThreadsafeFunction<String, String, String, Status, false>,
    >,
) -> napi::Result<Vec<LoadedSvgFile>> {
    let mut tasks = JoinSet::new();

    for (index, path) in paths.iter().cloned().enumerate() {
        tasks.spawn(async move {
            tokio::fs::read_to_string(&path)
                .await
                .map(|contents| (index, (path, contents)))
        });
    }

    let mut source_files = Vec::with_capacity(paths.len());

    while let Some(result) = tasks.join_next().await {
        let (index, (path, contents)) = result
            .map_err(|error| {
                Error::new(
                    Status::GenericFailure,
                    format!("Native SVG loading task failed: {error}"),
                )
            })?
            .map_err(|error| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to read source SVG file: {error}"),
                )
            })?;
        let glyph_name = glyph_name_from_path(&path, rename).await?;
        source_files.push((
            index,
            LoadedSvgFile {
                contents,
                glyph_name,
                path,
            },
        ));
    }

    source_files.sort_by_key(|(index, _)| *index);

    let source_files = source_files
        .into_iter()
        .map(|(_, source_file)| source_file)
        .collect::<Vec<_>>();

    validate_glyph_names(&source_files)?;

    Ok(source_files)
}

fn validate_glyph_names(source_files: &[LoadedSvgFile]) -> napi::Result<()> {
    let mut seen_names = HashSet::with_capacity(source_files.len());

    for source_file in source_files {
        if !seen_names.insert(source_file.glyph_name.clone()) {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "The glyph name \"{}\" must be unique.",
                    source_file.glyph_name
                ),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use napi::Status;

    use super::{
        resolve_generate_webfonts_options, resolved_font_types, validate_font_type_order,
        validate_generate_webfonts_options, woff,
    };
    use crate::{ttf::generate_ttf_font_bytes, FontType, GenerateWebfontsOptions};

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

        let result = woff::ttf_to_woff2(&ttf_result).expect("woff2 generation should succeed");

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

        assert_eq!(error.status, Status::InvalidArg);
        assert_eq!(
            error.reason.as_str(),
            "Invalid font type order: 'woff' is not present in 'types'."
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

        assert_eq!(error.status, Status::InvalidArg);
        assert_eq!(error.reason.as_str(), "\"options.dest\" is empty.");
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

        assert_eq!(error.status, Status::InvalidArg);
        assert_eq!(error.reason.as_str(), "\"options.files\" is empty.");
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

        assert_eq!(error.status, Status::InvalidArg);
        assert_eq!(
            error.reason.as_str(),
            "\"options.cssTemplate\" must not be empty."
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

        assert_eq!(error.status, Status::InvalidArg);
        assert_eq!(
            error.reason.as_str(),
            "\"options.htmlTemplate\" must not be empty."
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

        assert_eq!(error.status, Status::InvalidArg);
        assert!(error.reason.contains("cssTemplate"));
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

        assert_eq!(error.status, Status::InvalidArg);
        assert!(error.reason.contains("htmlTemplate"));
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
