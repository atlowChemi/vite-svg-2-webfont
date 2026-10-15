//! Browser-only, filesystem-free bindings. The worker owns each session; output arrays
//! are copied into JavaScript-owned memory before transfer to the caller.

use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use webfont_generator::{
    FontType, GenerateWebfontsFromSourcesOptions, GenerateWebfontsFromSourcesResult, OutputKind,
    SourceChange, SourceMembership, TtfFormatOptions, generate_from_sources_sync,
};

#[cfg(all(test, target_arch = "wasm32"))]
mod tests;

fn error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}

/// A retained engine result. The TypeScript API owns this object inside its worker.
#[wasm_bindgen]
pub struct FontSession(GenerateWebfontsFromSourcesResult);

#[wasm_bindgen]
impl FontSession {
    /// Decode and validate memory options. An omitted timestamp uses the browser clock,
    /// captured once so incremental updates retain the same font metadata.
    #[wasm_bindgen(constructor)]
    pub fn new(options: &str) -> Result<FontSession, JsValue> {
        let mut options: GenerateWebfontsFromSourcesOptions =
            serde_json::from_str(options).map_err(error)?;
        let ttf = options
            .format_options
            .get_or_insert_default()
            .ttf
            .get_or_insert(TtfFormatOptions {
                copyright: None,
                description: None,
                ts: None,
                url: None,
                version: None,
            });
        if ttf.ts.is_none() {
            let now = js_sys::Date::now() / 1000.0;
            if !now.is_finite() {
                return Err(error("Browser clock returned an invalid timestamp"));
            }
            ttf.ts = Some(now.floor() as i64);
        }
        generate_from_sources_sync(options).map(Self).map_err(error)
    }

    /// Apply a source batch with complete final membership. Ordinary errors retain the result.
    pub fn regenerate(&mut self, membership: &str, changes: &str) -> Result<(), JsValue> {
        let membership: SourceMembership = serde_json::from_str(membership).map_err(error)?;
        let changes: Vec<SourceChange> = serde_json::from_str(changes).map_err(error)?;
        self.0.regenerate_sync(membership, changes).map_err(error)
    }

    /// Copy a requested font without rendering companion templates.
    pub fn font(&self, format: &str) -> Result<Option<Uint8Array>, JsValue> {
        let format: FontType = serde_json::from_value(format.into()).map_err(error)?;
        let bytes = match format {
            FontType::Svg => self.0.svg_string().map(str::as_bytes),
            FontType::Ttf => self.0.ttf_bytes(),
            FontType::Woff => self.0.woff_bytes(),
            FontType::Woff2 => self.0.woff2_bytes(),
            FontType::Eot => self.0.eot_bytes(),
        };
        Ok(bytes.map(Uint8Array::from))
    }

    /// Render CSS lazily, even when CSS collection is disabled.
    pub fn css(&self) -> Result<String, JsValue> {
        self.0
            .generate_css()
            .map(|text| text.to_string())
            .map_err(error)
    }

    /// Render HTML lazily, reusing the engine's CSS cache.
    pub fn html(&self) -> Result<String, JsValue> {
        self.0
            .generate_html()
            .map(|text| text.to_string())
            .map_err(error)
    }

    /// Copy requested fonts and enabled companions into an object keyed by output kind.
    pub fn outputs(&self) -> Result<Object, JsValue> {
        let result = Object::new();
        for output in self.0.collect_outputs().map_err(error)? {
            let kind = match output.kind {
                OutputKind::Font(font) => font.as_extension(),
                OutputKind::Css => "css",
                OutputKind::Html => "html",
            };
            Reflect::set(
                &result,
                &kind.into(),
                &Uint8Array::from(output.contents.as_bytes()),
            )?;
        }
        Ok(result)
    }
}
