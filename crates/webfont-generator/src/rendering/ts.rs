use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::input::ResolvedGenerationOptions;

#[cfg(test)]
mod tests;

/// Render metadata directly: no template acquisition, compilation, CSS, or HTML rendering.
pub(crate) fn render_ts<'a>(
    options: &ResolvedGenerationOptions,
    names: impl Iterator<Item = &'a str>,
    variants: bool,
    css_context: Option<&Map<String, Value>>,
) -> String {
    let setting = |key, default| {
        css_context
            .or(options.template_options.as_ref())
            .map_or(default, |context| super::ctx_str(context, key, default))
    };
    let base = setting("baseSelector", ".icon");
    let prefix = setting("classPrefix", "icon-");
    let mut seen = HashSet::new();
    let names: Vec<_> = names
        .filter(|name| !variants || seen.insert(*name))
        .collect();
    format!(
        "export const iconBaseSelector = {} as const;\nexport const iconClassPrefix = {} as const;\nexport const icons = {} as const;\n\nexport type IconName = (typeof icons)[number];\n",
        serde_json::to_string(base).unwrap(),
        serde_json::to_string(prefix).unwrap(),
        serde_json::to_string_pretty(&names).unwrap(),
    )
}
