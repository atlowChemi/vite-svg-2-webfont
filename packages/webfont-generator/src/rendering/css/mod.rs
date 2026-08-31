mod context;
mod dependencies;
mod hash;
mod render;

#[cfg(feature = "napi")]
pub(crate) use context::apply_context_function;
pub(crate) use context::{
    SharedTemplateData, build_css_context, build_css_context_with_fonts_url, make_src,
};
pub(crate) use dependencies::{TemplateDependencies, template_dependencies};
pub(super) use render::render_css_with_context;
pub(crate) use render::{render_css_with_hbs_context, render_css_with_src_mutate};

#[cfg(test)]
use context::{make_ctx, make_urls};
#[cfg(test)]
use hash::calc_hash;
#[cfg(test)]
use render::render_default_css;

#[cfg(test)]
mod tests;
