mod context;
mod dependencies;
mod hash;
mod render;

pub(crate) use context::{
    SharedTemplateData, build_css_context, build_css_context_with_fonts_url,
    make_content_css_context, make_src, make_urls,
};
pub(crate) use dependencies::{TemplateDependencies, template_dependencies};
pub(crate) use render::render_css_with_context;
pub(crate) use render::{render_css_with_hbs_context, render_css_with_src_mutate};

#[cfg(test)]
use context::make_ctx;
pub(crate) use hash::calc_content_hash;
pub(super) use hash::calc_hash;

#[cfg(test)]
mod tests;
