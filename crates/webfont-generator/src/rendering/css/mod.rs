mod context;
mod dependencies;
mod hash;
mod render;

pub(crate) use context::SharedTemplateData;
#[cfg(feature = "in_memory")]
pub(crate) use context::make_content_css_context;
#[cfg(feature = "fs")]
pub(crate) use context::make_src;
#[cfg(any(feature = "in_memory", test))]
pub(crate) use context::make_urls;
#[cfg(feature = "fs")]
pub(crate) use context::{build_css_context, build_css_context_with_fonts_url};
pub(crate) use dependencies::{TemplateDependencies, template_dependencies};
pub(crate) use render::render_css_with_context;
#[cfg(feature = "fs")]
pub(crate) use render::{render_css_with_hbs_context, render_css_with_src_mutate};

#[cfg(all(test, feature = "fs"))]
use context::make_ctx;
#[cfg(feature = "in_memory")]
pub(crate) use hash::calc_content_hash;
#[cfg(feature = "fs")]
pub(super) use hash::calc_hash;

#[cfg(all(test, feature = "fs"))]
mod tests;
