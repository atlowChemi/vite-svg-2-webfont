use std::fmt::Write as _;
use std::io::Error;

use serde_json::{Map, Value};

use super::context::SharedTemplateData;
use crate::rendering::shared::to_io_err;

/// Render CSS using a pre-built Handlebars Context (no serialization).
/// Falls back to the hot-path renderer when no custom template is configured.
pub(crate) fn render_css_with_hbs_context(
    shared: &SharedTemplateData,
    hbs_ctx: &handlebars::Context,
    map_ctx: &Map<String, Value>,
) -> Result<String, Error> {
    match shared.css_registry()? {
        Some(registry) => registry
            .render_with_context("css", hbs_ctx)
            .map_err(to_io_err),
        None => Ok(render_default_css(map_ctx)),
    }
}

/// Render CSS from a Map context (used during init when no pre-built hbs Context exists yet).
pub(in crate::rendering) fn render_css_with_context(
    shared: &SharedTemplateData,
    ctx: &Map<String, Value>,
) -> Result<String, Error> {
    match shared.css_registry()? {
        Some(registry) => registry.render("css", ctx).map_err(to_io_err),
        None => Ok(render_default_css(ctx)),
    }
}

/// Render CSS with a different `src` value by mutating the Context in place,
/// rendering, then restoring the original value. Zero allocation.
/// Falls back to the hot-path renderer when no custom template is configured.
pub(crate) fn render_css_with_src_mutate(
    shared: &SharedTemplateData,
    hbs_ctx: &mut handlebars::Context,
    map_ctx: &Map<String, Value>,
    src: &str,
) -> Result<String, Error> {
    match shared.css_registry()? {
        Some(registry) => crate::rendering::shared::render_with_field_swap(
            hbs_ctx,
            "src",
            Value::String(src.to_owned()),
            |ctx| registry.render_with_context("css", ctx).map_err(to_io_err),
        ),
        None => Ok(render_default_css_inner(
            map_ctx,
            crate::rendering::ctx_str(map_ctx, "fontName", ""),
            src,
        )),
    }
}

pub(super) fn render_default_css(ctx: &Map<String, Value>) -> String {
    render_default_css_inner(
        ctx,
        crate::rendering::ctx_str(ctx, "fontName", ""),
        crate::rendering::ctx_str(ctx, "src", ""),
    )
}

fn render_default_css_inner(ctx: &Map<String, Value>, font_name: &str, src: &str) -> String {
    let base_selector = crate::rendering::ctx_str(ctx, "baseSelector", ".icon");
    let class_prefix = crate::rendering::ctx_str(ctx, "classPrefix", "icon-");
    let codepoints = ctx.get("codepoints").and_then(|v| v.as_object());

    let codepoint_count = codepoints.map_or(0, |c| c.len());
    let mut result = String::with_capacity(256 + codepoint_count * 60);

    _ = write!(
        result,
        "@font-face {{\n\tfont-family: \"{font_name}\";\n\tfont-display: block;\n\tsrc: {src};\n}}\n\n"
    );
    _ = write!(result, "{base_selector} {{\n\tline-height: 1;\n}}\n\n");
    _ = write!(
        result,
        "{base_selector}:before {{\n\tfont-family: {font_name} !important;\n\tfont-style: normal;\n\tfont-weight: normal !important;\n\tvertical-align: top;\n}}\n\n"
    );

    if let Some(codepoints) = codepoints {
        for (name, value) in codepoints {
            let code = value.as_str().unwrap_or("");
            _ = write!(
                result,
                ".{class_prefix}{name}:before {{\n\tcontent: \"\\{code}\";\n}}\n"
            );
        }
    }

    result
}
