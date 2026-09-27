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

    /// Resolve one glyph name per path, or use the engine's defaults.
    fn rename(
        &self,
        _paths: Vec<String>,
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
