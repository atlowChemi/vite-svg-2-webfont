//! Node-specific callback transport and exported result wrappers.
mod conversions;
mod result;
mod types;

use napi::Status;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use serde_json::{Map, Value};
use webfont_generator::GenerationHooks;

pub use result::GenerateWebfontsResult;
pub use types::*;

type RenameFunction = ThreadsafeFunction<Vec<String>, Vec<String>, Vec<String>, Status, false>;
type ContextFunction =
    ThreadsafeFunction<Map<String, Value>, Map<String, Value>, Map<String, Value>, Status, false>;

struct NodeHooks {
    rename: Option<RenameFunction>,
    css_context: Option<ContextFunction>,
    html_context: Option<ContextFunction>,
}

impl GenerationHooks for NodeHooks {
    type Error = napi::Error;

    async fn rename(&self, paths: &[String]) -> napi::Result<Option<Vec<String>>> {
        let Some(rename) = &self.rename else {
            return Ok(None);
        };
        let count = paths.len();
        let names = rename.call_async_catch(paths.to_vec()).await?;
        if names.len() != count {
            return Err(napi::Error::new(
                Status::InvalidArg,
                "rename callback returned an unexpected number of glyph names",
            ));
        }
        Ok(Some(names))
    }

    fn has_css_context(&self) -> bool {
        self.css_context.is_some()
    }
    fn has_html_context(&self) -> bool {
        self.html_context.is_some()
    }

    async fn css_context(&self, context: Map<String, Value>) -> napi::Result<Map<String, Value>> {
        apply_context(context, self.css_context.as_ref()).await
    }
    async fn html_context(&self, context: Map<String, Value>) -> napi::Result<Map<String, Value>> {
        apply_context(context, self.html_context.as_ref()).await
    }
}

async fn apply_context(
    context: Map<String, Value>,
    callback: Option<&ContextFunction>,
) -> napi::Result<Map<String, Value>> {
    match callback {
        Some(callback) => callback
            .call_async(context)
            .await
            .map_err(result::to_napi_err),
        None => Ok(context),
    }
}

/// Generate a webfont from a set of SVG files.
///
/// Loads the SVGs listed in `options.files`, builds the configured
/// `options.types` formats, optionally writes them (along with the CSS and
/// HTML preview) to `options.dest`, and returns a `GenerateWebfontsResult`
/// holding the font bytes and template-rendering methods.
///
/// Multi-variant input generates one shared variable font per requested modern format.
///
/// Optional callbacks:
/// - `rename(paths)` — derive custom glyph names for the batch of SVG file paths.
/// - `cssContext(ctx)` — mutate the Handlebars context before CSS rendering;
///   return the (possibly mutated) context.
/// - `htmlContext(ctx)` — same, but for the HTML preview.
#[napi]
// NAPI's TypeScript generator reads the callback syntax rather than resolving Rust aliases.
#[allow(clippy::type_complexity)]
pub async fn generate_webfonts(
    options: GenerateWebfontsOptions,
    rename: Option<ThreadsafeFunction<Vec<String>, Vec<String>, Vec<String>, Status, false>>,
    css_context: Option<
        ThreadsafeFunction<
            Map<String, Value>,
            Map<String, Value>,
            Map<String, Value>,
            Status,
            false,
        >,
    >,
    html_context: Option<
        ThreadsafeFunction<
            Map<String, Value>,
            Map<String, Value>,
            Map<String, Value>,
            Status,
            false,
        >,
    >,
) -> napi::Result<GenerateWebfontsResult> {
    let hooks = NodeHooks {
        rename,
        css_context,
        html_context,
    };
    webfont_generator::generate_with_hooks(options.try_into()?, &hooks)
        .await
        .map(GenerateWebfontsResult)
}
