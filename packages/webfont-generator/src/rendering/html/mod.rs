use std::fmt::Write as _;
use std::fs;
use std::io::Error;
use std::path::Path;

use crate::input::{LoadedSvgFile, ResolvedGenerateWebfontsOptions};
use crate::rendering::css::{SharedTemplateData, TemplateDependencies};

use crate::rendering::css::build_css_context_with_fonts_url;
use crate::rendering::css::render_css_with_context;
use crate::rendering::css::template_dependencies;
use crate::rendering::paths::{path_to_slashes, relative_path};
use crate::rendering::shared::to_io_err;
use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext, RenderErrorReason,
};
use serde_json::{Map, Value};

pub(crate) fn build_html_context(
    options: &ResolvedGenerateWebfontsOptions,
    shared: &SharedTemplateData,
    source_files: &[LoadedSvgFile],
    styles: Option<String>,
) -> Result<Map<String, Value>, Error> {
    let styles = match styles {
        Some(styles) => styles,
        None => {
            let html_fonts_url = html_css_fonts_url(options);
            let css_ctx = build_css_context_with_fonts_url(options, shared, Some(&html_fonts_url));
            render_css_with_context(shared, &css_ctx)?
        }
    };

    Ok(make_ctx(options, shared, source_files, styles))
}

/// Render HTML using a pre-built Handlebars Context (no serialization).
/// Falls back to the hot-path renderer when no custom template is configured.
pub(crate) fn render_html_with_hbs_context(
    cached_registry: Option<&Handlebars<'static>>,
    hbs_ctx: &handlebars::Context,
    map_ctx: &Map<String, Value>,
) -> Result<String, Error> {
    match cached_registry {
        Some(registry) => registry
            .render_with_context("html", hbs_ctx)
            .map_err(to_io_err),
        None => Ok(render_default_html(map_ctx)),
    }
}

#[cfg(test)]
fn render_html_with_context(
    options: &ResolvedGenerateWebfontsOptions,
    cached_registry: Option<&Handlebars<'static>>,
    ctx: &Map<String, Value>,
) -> Result<String, Error> {
    match cached_registry {
        Some(registry) => registry.render("html", ctx).map_err(to_io_err),
        None if options.html_template.is_some() => {
            let registry = build_html_registry(options)?
                .ok_or_else(|| to_io_err("HTML template path set but failed to compile"))?;
            registry.render("html", ctx).map_err(to_io_err)
        }
        None => Ok(render_default_html(ctx)),
    }
}

/// Pre-compile the HTML Handlebars template with the removePeriods helper.
/// Returns Ok(None) when no custom template is configured.
/// Returns Err when the template exists but fails to compile.
#[cfg(test)]
pub(crate) fn build_html_registry(
    options: &ResolvedGenerateWebfontsOptions,
) -> Result<Option<Handlebars<'static>>, Error> {
    Ok(build_html_registry_and_dependencies(options)?.0)
}

pub(crate) fn build_html_registry_and_dependencies(
    options: &ResolvedGenerateWebfontsOptions,
) -> Result<(Option<Handlebars<'static>>, TemplateDependencies), Error> {
    let path = match &options.html_template {
        Some(path) => path,
        None => return Ok((None, TemplateDependencies::html_default())),
    };
    let source = fs::read_to_string(path)?;
    let dependencies = template_dependencies(&source);
    let mut registry = Handlebars::new();
    registry.register_helper("removePeriods", Box::new(RemovePeriodsHelper));
    registry
        .register_template_string("html", &source)
        .map_err(|error| to_io_err(format!("Failed to compile HTML template: {error}")))?;
    Ok((Some(registry), dependencies))
}

#[inline]
pub(crate) fn render_default_html_with_styles(ctx: &Map<String, Value>, styles: &str) -> String {
    render_default_html_inner(ctx, styles)
}

fn render_default_html(ctx: &Map<String, Value>) -> String {
    render_default_html_inner(ctx, super::ctx_str(ctx, "styles", ""))
}

fn render_default_html_inner(ctx: &Map<String, Value>, styles: &str) -> String {
    let font_name = super::ctx_str(ctx, "fontName", "");
    let base_selector = super::ctx_str(ctx, "baseSelector", ".icon");
    let class_prefix = super::ctx_str(ctx, "classPrefix", "icon-");
    let names = ctx.get("names").and_then(|v| v.as_array());
    let base_class = base_selector.replace('.', "");

    let name_count = names.map_or(0, |n| n.len());
    let mut result = String::with_capacity(512 + name_count * 120);

    _ = write!(
        result,
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n\t<meta charset=\"UTF-8\">\n\t<title>{font_name}</title>\n\t<style>\n"
    );
    result.push_str("\t\tbody {\n\t\t\tfont-family: sans-serif;\n\t\t\tmargin: 0;\n\t\t\tpadding: 10px 20px;\n\t\t}\n\n");
    result.push_str("\t\t.preview {\n\t\t\tline-height: 2em;\n\t\t}\n\n");
    result.push_str("\t\t.preview__icon {\n\t\t\tdisplay: inline-block;\n\t\t\twidth: 32px;\n\t\t\ttext-align: center;\n\t\t}\n\n");
    _ = write!(
        result,
        "\t\t{base_selector} {{\n\t\t\tdisplay: inline-block;\n\t\t\tfont-size: 16px;\n\t\t}}\n\n"
    );
    _ = writeln!(result, "\t\t{styles}");
    result.push_str("\t</style>\n</head>\n<body>\n");
    _ = writeln!(result, "\t<h1>{font_name}</h1>");

    if let Some(names) = names {
        for name_value in names {
            let name = name_value.as_str().unwrap_or("");
            _ = write!(
                result,
                "\t<div class=\"preview\">\n\t\t<span class=\"preview__icon\">\n\t\t\t<span class=\"{base_class} {class_prefix}{name}\"></span>\n\t\t</span>\n\t\t<span>{name}</span>\n\t</div>\n"
            );
        }
    }

    result.push_str("</body>\n</html>\n");
    result
}

fn html_css_fonts_url(options: &ResolvedGenerateWebfontsOptions) -> String {
    let html_dir = Path::new(&options.html_dest)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    path_to_slashes(relative_path(html_dir, Path::new(&options.dest)))
}

fn make_ctx(
    options: &ResolvedGenerateWebfontsOptions,
    shared: &SharedTemplateData,
    source_files: &[LoadedSvgFile],
    styles: String,
) -> Map<String, Value> {
    let mut ctx = shared.template_options.clone();
    ctx.extend(Map::from_iter([
        (
            "codepoints".to_owned(),
            Value::Object(shared.codepoints_num.clone()),
        ),
        (
            "fontName".to_owned(),
            Value::String(options.font_name.clone()),
        ),
        (
            "names".to_owned(),
            Value::Array(
                source_files
                    .iter()
                    .map(|source_file| Value::String(source_file.glyph_name.clone()))
                    .collect(),
            ),
        ),
        ("styles".to_owned(), Value::String(styles)),
    ]));

    ctx
}

struct RemovePeriodsHelper;

impl HelperDef for RemovePeriodsHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        helper: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let selector = helper
            .param(0)
            .and_then(|value| value.value().as_str())
            .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("RemovePeriodsHelper", 0))?;

        out.write(&selector.replace('.', ""))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
