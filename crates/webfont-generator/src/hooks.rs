//! Runtime-independent asynchronous callbacks for language adapters.
use std::future::Future;

use serde_json::{Map, Value};

/// Async hooks used by [`crate::generate_with_hooks`].
///
/// Rename receives ordered paths after source loading, flattened in configured variant order.
/// Return `None` to derive names from file stems. Context hooks run only when their matching
/// `has_*_context` method returns true; CSS runs before HTML. Callback errors are preserved
/// through the associated error type, while engine failures are converted from I/O errors.
pub trait GenerationHooks: Send + Sync {
    /// Error returned to the caller, including converted engine errors.
    type Error: From<std::io::Error> + Send;

    /// Resolve one glyph name per borrowed path, or use the engine's defaults.
    /// Adapters that need an owned callback payload can copy paths inside this method.
    fn rename(
        &self,
        _paths: &[String],
    ) -> impl Future<Output = Result<Option<Vec<String>>, Self::Error>> + Send {
        async { Ok(None) }
    }

    /// Whether CSS context mutation is enabled.
    fn has_css_context(&self) -> bool {
        false
    }

    /// Whether HTML context mutation is enabled.
    fn has_html_context(&self) -> bool {
        false
    }

    /// Mutate the CSS template context before rendering.
    fn css_context(
        &self,
        context: Map<String, Value>,
    ) -> impl Future<Output = Result<Map<String, Value>, Self::Error>> + Send {
        async { Ok(context) }
    }

    /// Mutate the HTML context, including styles derived from the finalized CSS context.
    fn html_context(
        &self,
        context: Map<String, Value>,
    ) -> impl Future<Output = Result<Map<String, Value>, Self::Error>> + Send {
        async { Ok(context) }
    }
}

impl GenerationHooks for () {
    type Error = std::io::Error;
}

/// Adapt the existing synchronous Rust rename callback without copying input paths.
pub(crate) struct RenameHooks<'a>(pub Option<&'a (dyn Fn(&str) -> String + Send + Sync)>);

impl GenerationHooks for RenameHooks<'_> {
    type Error = std::io::Error;

    async fn rename(&self, paths: &[String]) -> std::io::Result<Option<Vec<String>>> {
        Ok(self
            .0
            .map(|rename| paths.iter().map(|path| rename(path)).collect()))
    }
}
