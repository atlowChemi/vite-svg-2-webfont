//! Browser-only, filesystem-free bindings. The worker owns each session; output arrays
//! are copied into JavaScript-owned memory before transfer to the caller.

use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use webfont_generator::{
    FontType, GenerateWebfontsFromSourcesOptions, GenerateWebfontsFromSourcesResult, OutputKind,
    SourceChange, SourceMembership, generate_from_sources_sync,
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
    /// captured once so incremental updates retain the same font metadata; fonts reused from
    /// `cache` keep the timestamp of the session that created them. `cache` enables the
    /// persistent cache with bytes from an earlier `cacheBytes()` (empty to start cold).
    #[wasm_bindgen(constructor)]
    pub fn new(options: &str, cache: Option<Vec<u8>>) -> Result<FontSession, JsValue> {
        let mut options: GenerateWebfontsFromSourcesOptions =
            serde_json::from_str(options).map_err(error)?;
        options.cache = cache;
        // WebAssembly has no system clock. The browser clock is only a fallback, so it doesn't
        // become part of cache keys and cached fonts keep the timestamp of the run that made them.
        let ts = options
            .format_options
            .as_ref()
            .and_then(|formats| formats.ttf.as_ref()?.ts);
        if ts.is_none() {
            let now = js_sys::Date::now() / 1000.0;
            if !now.is_finite() {
                return Err(error("Browser clock returned an invalid timestamp"));
            }
            options.fallback_ts = Some(now.floor() as i64);
        }
        generate_from_sources_sync(options).map(Self).map_err(error)
    }

    /// Apply a source batch with complete final membership. Ordinary errors retain the result.
    pub fn regenerate(&mut self, membership: &str, changes: &str) -> Result<(), JsValue> {
        let membership: SourceMembership = serde_json::from_str(membership).map_err(error)?;
        let changes: Vec<SourceChange> = serde_json::from_str(changes).map_err(error)?;
        self.0.regenerate_sync(membership, changes).map_err(error)
    }

    /// Append a source, requiring a target variant for families. Uses retained membership.
    pub fn add(&mut self, source: &str, variant: Option<String>) -> Result<(), JsValue> {
        self.0
            .add_sync(
                serde_json::from_str(source).map_err(error)?,
                variant.as_deref(),
            )
            .map_err(error)
    }

    /// Replace SVG text and optionally the name of an existing source in every design.
    pub fn update(&mut self, id: String, svg: String, name: Option<String>) -> Result<(), JsValue> {
        self.0.update_sync(id, svg, name).map_err(error)
    }

    /// Remove an existing source from every design, preserving the remaining order.
    pub fn remove(&mut self, id: String) -> Result<(), JsValue> {
        self.0.remove_sync(id).map_err(error)
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

    /// Encode the persistent cache, or `undefined` when caching is off or nothing changed since
    /// the bytes were loaded or last returned.
    #[wasm_bindgen(js_name = cacheBytes)]
    pub fn cache_bytes(&mut self) -> Option<Uint8Array> {
        self.0
            .cache_bytes()
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
    }

    /// Copy requested fonts and enabled companions into an object keyed by output kind.
    pub fn outputs(&self) -> Result<Object, JsValue> {
        let result = Object::new();
        for output in self.0.collect_outputs().map_err(error)? {
            let kind = match output.kind {
                OutputKind::Font(font) => font.as_extension(),
                OutputKind::Css => "css",
                OutputKind::Html => "html",
                OutputKind::Ts => "ts",
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
