mod css;
mod html;
mod paths;
mod result;
mod shared;

use serde_json::{Map, Value};

pub(crate) use css::render_css_with_hbs_context;
#[cfg(feature = "napi")]
pub(crate) use css::{SharedTemplateData, apply_context_function, build_css_context};
pub(crate) use html::render_html_with_hbs_context;
#[cfg(feature = "napi")]
pub(crate) use html::{build_html_context, build_html_registry_and_dependencies};
pub(crate) use result::{CachedTemplateData, CarriedRenderCache};

/// Extract a string value from a JSON Map context, with a default fallback.
#[inline]
pub(super) fn ctx_str<'a>(ctx: &'a Map<String, Value>, key: &str, default: &'a str) -> &'a str {
    ctx.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}
