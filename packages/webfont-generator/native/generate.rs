// NAPI copies function attributes into generated code, leaving function-level expectations unfulfilled.
#![expect(clippy::type_complexity)]

use napi::Status;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use serde_json::{Map, Value};

use super::{GenerateWebfontsOptions, GenerateWebfontsResult, NodeHooks};

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
// NAPI's TypeScript generator reads the callback syntax rather than resolving Rust aliases.
#[napi]
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
