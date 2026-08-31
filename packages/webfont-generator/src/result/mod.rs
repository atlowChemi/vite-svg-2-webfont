#[cfg(feature = "napi")]
#[path = "napi.rs"]
mod napi_adapter;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{Map, Value};

use crate::incremental::RegenerationState;
use crate::input::{LoadedSvgFile, ResolvedGenerateWebfontsOptions};
use crate::rendering::{CachedTemplateData, CarriedRenderCache};
#[cfg(feature = "bench")]
use crate::types::GlyphChange;

#[cfg(feature = "napi")]
pub(crate) use napi_adapter::to_napi_err;

/// Rendered bytes for each requested output format. Held by [`GenerateWebfontsResult`] and
/// produced by the generator's format pipeline; grouping them lets an incremental regenerate
/// refresh every format in a single assignment.
#[derive(Clone, Default)]
pub(crate) struct FontOutputs {
    pub(crate) svg_font: Option<Arc<String>>,
    pub(crate) ttf_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff2_font: Option<Arc<Vec<u8>>>,
    pub(crate) eot_font: Option<Arc<Vec<u8>>>,
}

/// Failure from asynchronous regeneration. Ordinary regeneration errors retain the input result
/// so callers can recover it with [`RegenerateError::into_result`] and retry.
pub struct RegenerateError {
    result: Option<Box<GenerateWebfontsResult>>,
    source: std::io::Error,
}

impl RegenerateError {
    pub(crate) fn new(result: Option<GenerateWebfontsResult>, source: std::io::Error) -> Self {
        Self {
            result: result.map(Box::new),
            source,
        }
    }

    /// Return the result that was consumed by the failed operation. This is `None` only if the
    /// blocking task was cancelled by the runtime before it could return the result.
    pub fn into_result(self) -> Option<GenerateWebfontsResult> {
        self.result.map(|result| *result)
    }

    /// Return both the recoverable result and the underlying I/O error.
    pub fn into_parts(self) -> (Option<GenerateWebfontsResult>, std::io::Error) {
        (self.result.map(|result| *result), self.source)
    }
}

impl std::fmt::Debug for RegenerateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegenerateError")
            .field("recoverable", &self.result.is_some())
            .field("source", &self.source)
            .finish()
    }
}

impl std::fmt::Display for RegenerateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for RegenerateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Result of a successful `generateWebfonts` call. Exposes the generated
/// font bytes (or `null` for formats that were not requested) and methods to
/// render the CSS and HTML preview.
#[cfg_attr(feature = "napi", napi_derive::napi)]
pub struct GenerateWebfontsResult {
    pub(crate) cached: OnceLock<Result<CachedTemplateData, String>>,
    /// Render-cache entries carried across an incremental `regenerate` to seed the rebuilt
    /// [`CachedTemplateData`], so CSS/HTML that
    /// doesn't depend on what changed isn't re-rendered. `None` for a normal build.
    pub(crate) carried_render: Option<CarriedRenderCache>,
    pub(crate) css_context: Option<Map<String, Value>>,
    pub(crate) fonts: FontOutputs,
    pub(crate) html_context: Option<Map<String, Value>>,
    pub(crate) options: Arc<ResolvedGenerateWebfontsOptions>,
    pub(crate) regeneration_state: Arc<Mutex<Option<RegenerationState>>>,
    pub(crate) source_files: Arc<Vec<LoadedSvgFile>>,
}

impl GenerateWebfontsResult {
    #[cfg(any(feature = "napi", feature = "bench"))]
    pub(crate) fn snapshot_for_regeneration(&self, state: RegenerationState) -> Self {
        Self {
            cached: OnceLock::new(),
            carried_render: self.render_cache_source(),
            css_context: self.css_context.clone(),
            fonts: self.fonts.clone(),
            html_context: self.html_context.clone(),
            options: self.options.clone(),
            regeneration_state: Arc::new(Mutex::new(Some(state))),
            source_files: self.source_files.clone(),
        }
    }

    pub(crate) fn take_regeneration_state(&self) -> std::io::Result<RegenerationState> {
        if !self.options.incremental {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "regenerate requires the font to be generated with `incremental` enabled.",
            ));
        }
        self.regeneration_state
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "This result is already regenerating or has been replaced.",
                )
            })
    }

    #[cfg(feature = "bench")]
    pub(crate) fn restore_regeneration_state(&self, state: RegenerationState) {
        *self.regeneration_state.lock().unwrap() = Some(state);
    }

    pub(crate) fn seed_written_outputs(&self, written_outputs: HashMap<String, [u8; 16]>) {
        if let Some(state) = self.regeneration_state.lock().unwrap().as_mut() {
            state.written_outputs = written_outputs;
        }
    }

    #[doc(hidden)]
    #[cfg(feature = "bench")]
    pub fn regenerate_owned_for_bench(
        &self,
        files: &[String],
        changes: &[(String, GlyphChange)],
    ) -> std::io::Result<Self> {
        let state = self.take_regeneration_state()?;
        let mut replacement = self.snapshot_for_regeneration(state);
        match replacement.regenerate(files, changes) {
            Ok(()) => Ok(replacement),
            Err(error) => {
                if let Ok(state) = replacement.take_regeneration_state() {
                    self.restore_regeneration_state(state);
                }
                Err(error)
            }
        }
    }

    /// Returns the EOT font bytes, if generated.
    pub fn eot_bytes(&self) -> Option<&[u8]> {
        self.fonts.eot_font.as_ref().map(|v| v.as_ref().as_slice())
    }

    /// Returns the SVG font string, if generated.
    pub fn svg_string(&self) -> Option<&str> {
        self.fonts.svg_font.as_ref().map(|v| v.as_ref().as_str())
    }

    /// Returns the TTF font bytes, if generated.
    pub fn ttf_bytes(&self) -> Option<&[u8]> {
        self.fonts.ttf_font.as_ref().map(|v| v.as_ref().as_slice())
    }

    /// Returns the WOFF font bytes, if generated.
    pub fn woff_bytes(&self) -> Option<&[u8]> {
        self.fonts.woff_font.as_ref().map(|v| v.as_ref().as_slice())
    }

    /// Returns the WOFF2 font bytes, if generated.
    pub fn woff2_bytes(&self) -> Option<&[u8]> {
        self.fonts
            .woff2_font
            .as_ref()
            .map(|v| v.as_ref().as_slice())
    }
}
