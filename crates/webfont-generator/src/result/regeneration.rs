use std::sync::Arc;

use crate::{GlyphChange, RegenerationFiles};

use super::GenerateWebfontsResult;

impl GenerateWebfontsResult {
    /// Rebuild on Tokio's blocking pool, preserving this result as a readable snapshot.
    ///
    /// Returns a replacement result on success. This result remains readable, but can no
    /// longer regenerate after it has been replaced. Concurrent regeneration attempts on
    /// the same result are rejected. On failure, its regeneration state is restored for
    /// retry, including reconciliation of any partially completed variant output writes.
    ///
    /// Pass `Some(changes)` to supply explicit changes, or `None` to infer them from the
    /// complete input membership. Requires generation with `incremental: true`.
    ///
    /// Unlike [`Self::regenerate_async`], this method borrows rather than consumes the
    /// receiver. Prefer the consuming API when Rust ownership can enforce replacement.
    /// This future is not cancellation-safe: dropping it does not stop blocking work or
    /// filesystem writes. If that work succeeds, this receiver remains replaced even
    /// though the new result can no longer be retrieved from the dropped future.
    pub async fn regenerate_snapshot_async(
        &self,
        files: RegenerationFiles,
        changes: Option<Vec<(String, GlyphChange)>>,
    ) -> std::io::Result<Self> {
        let state = self.take_regeneration_state()?;
        let original_sources = Arc::clone(&self.source_files);
        let is_variant = self.options.variants.is_some();
        let original_state = Arc::clone(&self.regeneration_state);
        let mut replacement = self.snapshot_for_regeneration(state);
        tokio::task::spawn_blocking(move || -> std::io::Result<Self> {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match changes {
                Some(changes) => replacement.regenerate(&files, &changes),
                None => replacement.regenerate_all(&files),
            }));
            if !matches!(result, Ok(Ok(()))) {
                let mut state = replacement.take_regeneration_state().ok();
                if let Some(state) = state.as_mut()
                    && is_variant
                    && !Arc::ptr_eq(&original_sources, &replacement.source_files)
                {
                    // Authoritative sources belong to the receiver, not this movable cache
                    // state. Discard caches for the failed replacement's committed sources.
                    state.variant_cache = None;
                    // The write map describes actual successful writes, including partial
                    // ones. A no-op retry must reconcile disk output with the old receiver.
                    state.variant_write_pending = replacement.options.write_files;
                }
                *original_state.lock().unwrap() = state;
            }
            match result {
                Ok(Ok(())) => Ok(replacement),
                Ok(Err(error)) => Err(error),
                Err(payload) => std::panic::resume_unwind(payload),
            }
        })
        .await
        .map_err(|error| {
            std::io::Error::other(format!("Native webfont regeneration task failed: {error}"))
        })?
    }
}
