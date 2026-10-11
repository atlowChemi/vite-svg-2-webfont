//! File-adapter template acquisition. Shared compilation only consumes text.
use std::sync::Arc;

use crate::input::{LoadedSvgSource, ResolvedFileGenerationOptions};

use super::css::SharedTemplateData;
use super::css::calc_hash;
use super::html::{HtmlTemplate, compile_html_template};

fn read_template(path: Option<&str>) -> std::io::Result<Option<Arc<str>>> {
    path.map(|path| std::fs::read_to_string(path).map(Arc::from))
        .transpose()
}

impl SharedTemplateData {
    /// Preserve file-mode acquisition timing: read CSS on shared-context initialization,
    /// then compile it only when a renderer first requests the registry.
    pub(crate) fn from_files(
        options: &ResolvedFileGenerationOptions,
        sources: &[LoadedSvgSource],
    ) -> std::io::Result<Self> {
        let source = read_template(options.css_template.as_deref())?;
        Ok(Self::new(
            &options.common,
            source,
            calc_hash(options, sources),
        ))
    }
}

pub(crate) fn load_html_template(
    options: &ResolvedFileGenerationOptions,
) -> std::io::Result<HtmlTemplate> {
    let source = read_template(options.html_template.as_deref())?;
    compile_html_template(source)
}
