pub(crate) mod css;
#[cfg(feature = "fs")]
mod files;
mod fingerprint;
pub(crate) mod html;
mod paths;
#[cfg(feature = "fs")]
mod result;
mod shared;
pub(crate) mod ts;

use serde_json::{Map, Value};

#[cfg(feature = "fs")]
pub(crate) use css::{SharedTemplateData, build_css_context};
#[cfg(feature = "fs")]
pub(crate) use files::load_html_template;
pub(crate) use fingerprint::font_settings_json;
#[cfg(feature = "fs")]
pub(crate) use html::build_html_context;
#[cfg(feature = "fs")]
pub(crate) use html::build_html_context_with_css;
#[cfg(feature = "fs")]
pub(crate) use result::{CachedTemplateData, CarriedRenderCache};

/// Extract a string value from a JSON Map context, with a default fallback.
#[inline]
pub(super) fn ctx_str<'a>(ctx: &'a Map<String, Value>, key: &str, default: &'a str) -> &'a str {
    ctx.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}
