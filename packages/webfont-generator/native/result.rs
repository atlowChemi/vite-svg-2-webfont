use std::collections::HashMap;

use napi::bindgen_prelude::Uint8Array;
use napi_derive::napi;

use crate::types::GlyphChangeEntry;
use webfont_generator::{FontType, GlyphChange};

/// Result of a successful `generateWebfonts` call. Exposes the generated
/// font bytes (or `null` for formats that were not requested) and methods to
/// render the CSS and HTML preview.
#[napi]
pub struct GenerateWebfontsResult(pub(crate) webfont_generator::GenerateWebfontsResult);

#[napi]
impl GenerateWebfontsResult {
    /// EOT font bytes, or `null` if EOT was not in `types`.
    #[napi(getter)]
    pub fn eot(&self) -> Option<Uint8Array> {
        self.0
            .eot_bytes()
            .map(|bytes| Uint8Array::from(bytes.to_vec()))
    }

    /// SVG font XML string, or `null` if SVG was not in `types`.
    #[napi(getter)]
    pub fn svg(&self) -> Option<String> {
        self.0.svg_string().map(str::to_owned)
    }

    /// TTF font bytes, or `null` if TTF was not in `types`.
    #[napi(getter)]
    pub fn ttf(&self) -> Option<Uint8Array> {
        self.0
            .ttf_bytes()
            .map(|bytes| Uint8Array::from(bytes.to_vec()))
    }

    /// WOFF2 font bytes, or `null` if WOFF2 was not in `types`.
    #[napi(getter)]
    pub fn woff2(&self) -> Option<Uint8Array> {
        self.0
            .woff2_bytes()
            .map(|bytes| Uint8Array::from(bytes.to_vec()))
    }

    /// WOFF font bytes, or `null` if WOFF was not in `types`.
    #[napi(getter)]
    pub fn woff(&self) -> Option<Uint8Array> {
        self.0
            .woff_bytes()
            .map(|bytes| Uint8Array::from(bytes.to_vec()))
    }

    /// Render the CSS string for this result. Pass `urls` to override the
    /// default font URLs in the `@font-face src:` descriptor. A supplied map is a complete
    /// override; omitted formats use empty URLs. The result is cached per `urls` value, so
    /// repeated calls with the same input are cheap.
    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_css(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        self.0.generate_css_pure(urls).map_err(to_napi_err)
    }

    /// Render the HTML preview string for this result. Pass `urls` to
    /// override font URLs in the embedded stylesheet. A supplied map is a complete override;
    /// omitted formats use empty URLs. The result is cached per `urls` value.
    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_html(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        self.0.generate_html_pure(urls).map_err(to_napi_err)
    }

    /// Rebuild the font after a batch of file changes, reusing cached glyph geometry for files
    /// whose contents are unchanged. Requires the font to have been generated with
    /// `incremental: true`. Supply `{ files: [...] }` for an ordinary font or
    /// `{ variants: [{ variant, files }, ...] }` with every configured design for a family.
    /// Each list is the complete file set after the changes, in the order a
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
        files: crate::types::RegenerationFileOptions,
        changes: Option<Vec<GlyphChangeEntry>>,
    ) -> napi::Result<()> {
        let files = parse_regeneration_files(files)?;
        let changes = parse_glyph_changes(changes)?;
        match changes {
            Some(changes) => self.0.regenerate(&files, &changes),
            None => self.0.regenerate_all(&files),
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
        files: crate::types::RegenerationFileOptions,
        changes: Option<Vec<GlyphChangeEntry>>,
    ) -> napi::Result<GenerateWebfontsResult> {
        let files = parse_regeneration_files(files)?;
        let changes = parse_glyph_changes(changes)?;
        self.0
            .regenerate_snapshot_async(files, changes)
            .await
            .map(Self)
            .map_err(to_napi_err)
    }
}

fn parse_regeneration_files(
    input: crate::types::RegenerationFileOptions,
) -> napi::Result<crate::RegenerationFiles> {
    match (input.files, input.variants) {
        (Some(files), None) => Ok(crate::RegenerationFiles::Single(files)),
        (None, Some(variants)) => Ok(crate::RegenerationFiles::Variants(
            variants.into_iter().map(Into::into).collect(),
        )),
        _ => Err(to_napi_err(
            "Regeneration input requires exactly one of files or variants.",
        )),
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
#[path = "result/tests.rs"]
mod tests;
