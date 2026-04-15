mod css;
mod html;

use serde_json::{Map, Value};

/// Extract a string value from a JSON Map context, with a default fallback.
#[inline]
pub(super) fn ctx_str<'a>(ctx: &'a Map<String, Value>, key: &str, default: &'a str) -> &'a str {
    ctx.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

#[cfg(feature = "napi")]
pub(crate) use css::apply_context_function;
pub(crate) use css::{
    SharedTemplateData, build_css_context, make_src, render_css_with_hbs_context,
    render_css_with_src_mutate,
};
pub(crate) use html::{
    build_html_context, build_html_registry, render_default_html_with_styles,
    render_html_with_hbs_context,
};
