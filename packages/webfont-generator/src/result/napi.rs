use std::collections::HashMap;
use std::sync::Arc;

use napi::bindgen_prelude::Uint8Array;
use napi_derive::napi;

use super::GenerateWebfontsResult;
use crate::types::{FontType, GlyphChange, GlyphChangeEntry};

#[napi]
impl GenerateWebfontsResult {
    /// EOT font bytes, or `null` if EOT was not in `types`.
    #[napi(getter)]
    pub fn eot(&self) -> Option<Uint8Array> {
        self.fonts
            .eot_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// SVG font XML string, or `null` if SVG was not in `types`.
    #[napi(getter)]
    pub fn svg(&self) -> Option<String> {
        self.fonts.svg_font.as_ref().map(|v| v.as_ref().clone())
    }

    /// TTF font bytes, or `null` if TTF was not in `types`.
    #[napi(getter)]
    pub fn ttf(&self) -> Option<Uint8Array> {
        self.fonts
            .ttf_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// WOFF2 font bytes, or `null` if WOFF2 was not in `types`.
    #[napi(getter)]
    pub fn woff2(&self) -> Option<Uint8Array> {
        self.fonts
            .woff2_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// WOFF font bytes, or `null` if WOFF was not in `types`.
    #[napi(getter)]
    pub fn woff(&self) -> Option<Uint8Array> {
        self.fonts
            .woff_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    /// Render the CSS string for this result. Pass `urls` to override the
    /// default font URLs in the `@font-face src:` descriptor (only the keys
    /// you supply are overridden). The result is cached per `urls` value, so
    /// repeated calls with the same input are cheap.
    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_css(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        self.generate_css_pure(urls).map_err(to_napi_err)
    }

    /// Render the HTML preview string for this result. Pass `urls` to
    /// override font URLs in the embedded stylesheet (only the keys you
    /// supply are overridden). The result is cached per `urls` value.
    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_html(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        self.generate_html_pure(urls).map_err(to_napi_err)
    }

    /// Rebuild the font after a batch of file changes, reusing cached glyph geometry for files
    /// whose contents are unchanged. Requires the font to have been generated with
    /// `incremental: true`. `files` is the complete file set after the changes, in the order a
    /// fresh build would use (e.g. the glob result) — the rebuilt glyphs are ordered to match it,
    /// so the output bytes are identical to a fresh `generateWebfonts` of that set. `changes`
    /// describes the affected files: added/changed files are re-read from disk; any file absent
    /// from `files` is dropped. Omit `changes` to re-read/hash every current file and infer
    /// added/changed/removed paths from `files`. Every requested format is refreshed in memory,
    /// and — like `generateWebfonts` — when the result was built with `writeFiles` enabled the
    /// refreshed fonts are written to disk too, while CSS/HTML companion files are skipped if their
    /// rendered bytes are unchanged since the last write.
    #[napi(js_name = "regenerate")]
    pub fn regenerate_from_js(
        &mut self,
        files: Vec<String>,
        changes: Option<Vec<GlyphChangeEntry>>,
    ) -> napi::Result<()> {
        let changes = parse_glyph_changes(changes)?;
        match changes {
            Some(changes) => self.regenerate(&files, &changes),
            None => self.regenerate_all(&files),
        }
        .map_err(to_napi_err)
    }

    /// Rebuild off the Node.js event loop and resolve with a replacement result. The receiver
    /// remains readable and unchanged while regeneration runs and after failure. Assign the
    /// resolved result before starting another regeneration. Overlapping calls on the same result
    /// lineage are rejected, and disk writes remain non-transactional.
    #[napi(js_name = "regenerateAsync")]
    pub async fn regenerate_async_from_js(
        &self,
        files: Vec<String>,
        changes: Option<Vec<GlyphChangeEntry>>,
    ) -> napi::Result<GenerateWebfontsResult> {
        let changes = parse_glyph_changes(changes)?;
        let state = self.take_regeneration_state().map_err(to_napi_err)?;
        let original_state = Arc::clone(&self.regeneration_state);
        let mut replacement = self.snapshot_for_regeneration(state);
        tokio::task::spawn_blocking(move || -> std::io::Result<GenerateWebfontsResult> {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match changes {
                Some(changes) => replacement.regenerate(&files, &changes),
                None => replacement.regenerate_all(&files),
            }));
            match result {
                Ok(Ok(())) => Ok(replacement),
                Ok(Err(error)) => {
                    if let Ok(state) = replacement.take_regeneration_state() {
                        *original_state.lock().unwrap() = Some(state);
                    }
                    Err(error)
                }
                Err(payload) => {
                    if let Ok(state) = replacement.take_regeneration_state() {
                        *original_state.lock().unwrap() = Some(state);
                    }
                    std::panic::resume_unwind(payload)
                }
            }
        })
        .await
        .map_err(|error| {
            napi::Error::from_reason(format!("Native webfont regeneration task failed: {error}"))
        })?
        .map_err(to_napi_err)
    }
}

pub(crate) fn to_napi_err(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::new(napi::Status::GenericFailure, error.to_string())
}

fn parse_glyph_changes(
    changes: Option<Vec<GlyphChangeEntry>>,
) -> napi::Result<Option<Vec<(String, GlyphChange)>>> {
    changes
        .map(|changes| {
            changes
                .into_iter()
                .map(|entry| {
                    let change = match entry.change_type.as_str() {
                        "added" => GlyphChange::Added { name: entry.name },
                        "changed" => GlyphChange::Changed { name: entry.name },
                        "removed" => GlyphChange::Removed,
                        other => {
                            return Err(napi::Error::from_reason(format!(
                                "Unknown changeType '{other}'; expected 'added', 'changed', or 'removed'."
                            )));
                        }
                    };
                    Ok((entry.path, change))
                })
                .collect()
        })
        .transpose()
}

fn parse_native_urls(urls: HashMap<String, String>) -> napi::Result<HashMap<FontType, String>> {
    urls.into_iter()
        .filter_map(|(font_type, url)| {
            let font_type = match font_type.as_str() {
                "svg" => Some(FontType::Svg),
                "ttf" => Some(FontType::Ttf),
                "eot" => Some(FontType::Eot),
                "woff" => Some(FontType::Woff),
                "woff2" => Some(FontType::Woff2),
                _ => None,
            }?;

            Some(Ok((font_type, url)))
        })
        .collect::<napi::Result<HashMap<FontType, String>>>()
}

#[cfg(test)]
#[path = "napi/tests.rs"]
mod tests;
