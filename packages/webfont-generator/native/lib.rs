//! Node-specific callback transport and exported result wrappers.
mod conversions;
mod generate;
mod result;
mod types;

use napi::Status;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use serde_json::{Map, Value};
use webfont_generator::GenerationHooks;

pub use generate::generate_webfonts;
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

/// Files and bytes removed by `clearCache`.
#[napi(object)]
pub struct ClearCacheReport {
    /// Number of cache files removed.
    pub files_removed: u32,
    /// Total size of the removed files in bytes.
    pub bytes_removed: f64,
}

pub struct ClearCacheTask(String);

impl napi::Task for ClearCacheTask {
    type Output = webfont_generator::ClearCacheReport;
    type JsValue = ClearCacheReport;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        webfont_generator::clear_cache(&self.0)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(ClearCacheReport {
            files_removed: u32::try_from(output.files_removed).unwrap_or(u32::MAX),
            bytes_removed: output.bytes_removed as f64,
        })
    }
}

/// Remove every file the generation cache created in `cacheLocation` (see the `cacheLocation`
/// option), including leftovers from interrupted writes. Other files are left untouched, and a
/// missing directory resolves with zero counts.
#[napi(ts_return_type = "Promise<ClearCacheReport>")]
pub fn clear_cache(cache_location: String) -> napi::bindgen_prelude::AsyncTask<ClearCacheTask> {
    napi::bindgen_prelude::AsyncTask::new(ClearCacheTask(cache_location))
}
