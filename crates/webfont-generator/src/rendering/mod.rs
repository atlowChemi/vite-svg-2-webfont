mod css;
mod files;
mod html;
mod paths;
mod result;
mod shared;

use serde_json::{Map, Value};

pub(crate) use css::{SharedTemplateData, build_css_context};
pub(crate) use files::load_html_template;
pub(crate) use html::build_html_context;
pub(crate) use html::build_html_context_with_css;
pub(crate) use result::{CachedTemplateData, CarriedRenderCache};

/// Extract a string value from a JSON Map context, with a default fallback.
#[inline]
pub(super) fn ctx_str<'a>(ctx: &'a Map<String, Value>, key: &str, default: &'a str) -> &'a str {
    ctx.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}
