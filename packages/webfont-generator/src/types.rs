use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use napi::bindgen_prelude::Uint8Array;
use napi_derive::napi;
use serde_json::{Map, Value};

use crate::templates::{
    build_css_context, build_html_context, build_html_registry, make_src,
    render_css_with_hbs_context, render_css_with_src_mutate, render_default_html_with_styles,
    render_html_with_hbs_context, SharedTemplateData,
};
use crate::util::to_napi_err;

#[napi(string_enum = "lowercase")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontType {
    Svg,
    Ttf,
    Eot,
    Woff,
    Woff2,
}

impl FontType {
    /// Returns the CSS `format()` value (e.g., "truetype", "woff2").
    #[inline]
    pub fn css_format(self) -> &'static str {
        match self {
            FontType::Svg => "svg",
            FontType::Ttf => "truetype",
            FontType::Eot => "embedded-opentype",
            FontType::Woff => "woff",
            FontType::Woff2 => "woff2",
        }
    }

    /// Returns the file extension (e.g., "svg", "ttf", "woff2").
    #[inline]
    pub fn as_extension(self) -> &'static str {
        match self {
            FontType::Svg => "svg",
            FontType::Ttf => "ttf",
            FontType::Eot => "eot",
            FontType::Woff => "woff",
            FontType::Woff2 => "woff2",
        }
    }
}

#[napi(object)]
#[derive(Clone, Default)]
pub struct SvgFormatOptions {
    pub center_vertically: Option<bool>,
    pub font_id: Option<String>,
    pub metadata: Option<String>,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
}

#[napi(object)]
#[derive(Clone)]
pub struct TtfFormatOptions {
    pub copyright: Option<String>,
    pub description: Option<String>,
    pub ts: Option<i64>,
    pub url: Option<String>,
    pub version: Option<String>,
}

#[napi(object)]
#[derive(Clone)]
pub struct WoffFormatOptions {
    pub metadata: Option<String>,
}

#[napi(object)]
#[derive(Clone, Default)]
pub struct FormatOptions {
    pub svg: Option<SvgFormatOptions>,
    pub ttf: Option<TtfFormatOptions>,
    pub woff: Option<WoffFormatOptions>,
}

#[napi(object)]
#[derive(Clone, Default)]
pub struct GenerateWebfontsOptions {
    pub ascent: Option<f64>,
    pub center_horizontally: Option<bool>,
    pub center_vertically: Option<bool>,
    pub css: Option<bool>,
    pub css_dest: Option<String>,
    pub css_template: Option<String>,
    pub codepoints: Option<HashMap<String, u32>>,
    pub css_fonts_url: Option<String>,
    pub descent: Option<f64>,
    pub dest: String,
    pub files: Vec<String>,
    pub fixed_width: Option<bool>,
    pub format_options: Option<FormatOptions>,
    pub html: Option<bool>,
    pub html_dest: Option<String>,
    pub html_template: Option<String>,
    pub font_height: Option<f64>,
    pub font_name: Option<String>,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub ligature: Option<bool>,
    pub normalize: Option<bool>,
    pub order: Option<Vec<FontType>>,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
    pub round: Option<f64>,
    pub start_codepoint: Option<u32>,
    pub template_options: Option<Map<String, Value>>,
    pub types: Option<Vec<FontType>>,
    pub write_files: Option<bool>,
}

pub(crate) const DEFAULT_FONT_TYPES: [FontType; 3] =
    [FontType::Eot, FontType::Woff, FontType::Woff2];

pub(crate) const DEFAULT_FONT_ORDER: [FontType; 5] = [
    FontType::Eot,
    FontType::Woff2,
    FontType::Woff,
    FontType::Ttf,
    FontType::Svg,
];

pub(crate) fn resolved_font_types(options: &GenerateWebfontsOptions) -> Vec<FontType> {
    match &options.types {
        Some(types) => types.clone(),
        None => DEFAULT_FONT_TYPES.to_vec(),
    }
}

pub(crate) struct ResolvedGenerateWebfontsOptions {
    pub ascent: Option<f64>,
    pub center_horizontally: Option<bool>,
    pub center_vertically: Option<bool>,
    pub css: bool,
    pub css_dest: String,
    pub css_template: Option<String>,
    pub codepoints: BTreeMap<String, u32>,
    pub css_fonts_url: Option<String>,
    pub descent: Option<f64>,
    pub dest: String,
    pub files: Vec<String>,
    pub fixed_width: Option<bool>,
    pub format_options: Option<FormatOptions>,
    pub html: bool,
    pub html_dest: String,
    pub html_template: Option<String>,
    pub font_height: Option<f64>,
    pub font_name: String,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub ligature: bool,
    pub normalize: bool,
    pub order: Vec<FontType>,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
    pub round: Option<f64>,
    pub start_codepoint: u32,
    pub template_options: Option<Map<String, Value>>,
    pub types: Vec<FontType>,
    pub write_files: bool,
}

pub(crate) struct LoadedSvgFile {
    pub contents: String,
    pub glyph_name: String,
    pub path: String,
}

/// Caches the last rendered CSS/HTML result for repeated calls with the same urls.
#[derive(Default)]
pub(crate) struct RenderCache {
    /// Result of generateCss() with no urls (computed once).
    css_no_urls: Option<String>,
    /// Last generateCss(urls) result.
    css_last_urls: Option<HashMap<FontType, String>>,
    css_last_result: Option<String>,
    /// Result of generateHtml() with no urls (computed once).
    html_no_urls: Option<String>,
    /// Last generateHtml(urls) result.
    html_last_urls: Option<HashMap<FontType, String>>,
    html_last_result: Option<String>,
}

pub(crate) struct CachedTemplateData {
    pub shared: SharedTemplateData,
    pub css_context: Map<String, Value>,
    pub css_hbs_context: Mutex<handlebars::Context>,
    pub html_context: Map<String, Value>,
    pub html_hbs_context: Mutex<handlebars::Context>,
    pub html_registry: Option<handlebars::Handlebars<'static>>,
    pub(crate) render_cache: Mutex<RenderCache>,
}

#[napi]
pub struct GenerateWebfontsResult {
    pub(crate) css_context: Option<Map<String, Value>>,
    pub(crate) eot_font: Option<Arc<Vec<u8>>>,
    pub(crate) html_context: Option<Map<String, Value>>,
    pub(crate) options: ResolvedGenerateWebfontsOptions,
    pub(crate) source_files: Vec<LoadedSvgFile>,
    pub(crate) svg_font: Option<Arc<String>>,
    pub(crate) ttf_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff2_font: Option<Arc<Vec<u8>>>,
    pub(crate) woff_font: Option<Arc<Vec<u8>>>,
    pub(crate) cached: OnceLock<Result<CachedTemplateData, String>>,
}

#[napi]
impl GenerateWebfontsResult {
    #[napi(getter)]
    pub fn eot(&self) -> Option<Uint8Array> {
        self.eot_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    #[napi(getter)]
    pub fn svg(&self) -> Option<String> {
        self.svg_font.as_ref().map(|v| v.as_ref().clone())
    }

    #[napi(getter)]
    pub fn ttf(&self) -> Option<Uint8Array> {
        self.ttf_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    #[napi(getter)]
    pub fn woff2(&self) -> Option<Uint8Array> {
        self.woff2_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    #[napi(getter)]
    pub fn woff(&self) -> Option<Uint8Array> {
        self.woff_font
            .as_ref()
            .map(|v| Uint8Array::from(v.as_ref().clone()))
    }

    pub(crate) fn get_cached(&self) -> napi::Result<&CachedTemplateData> {
        self.cached
            .get_or_init(|| {
                let shared = SharedTemplateData::new(&self.options, &self.source_files)
                    .map_err(|e| e.to_string())?;
                let css_context = match &self.css_context {
                    Some(ctx) => ctx.clone(),
                    None => build_css_context(&self.options, &shared),
                };
                let html_context = match &self.html_context {
                    Some(ctx) => ctx.clone(),
                    None => build_html_context(&self.options, &shared, &self.source_files, None)
                        .map_err(|e| e.to_string())?,
                };
                let html_registry =
                    build_html_registry(&self.options).map_err(|e| e.to_string())?;
                let css_hbs_context =
                    handlebars::Context::wraps(&css_context).map_err(|e| e.to_string())?;
                let html_hbs_context =
                    handlebars::Context::wraps(&html_context).map_err(|e| e.to_string())?;
                Ok(CachedTemplateData {
                    shared,
                    css_context,
                    css_hbs_context: Mutex::new(css_hbs_context),
                    html_context,
                    html_hbs_context: Mutex::new(html_hbs_context),
                    html_registry,
                    render_cache: Mutex::new(RenderCache::default()),
                })
            })
            .as_ref()
            .map_err(to_napi_err)
    }

    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_css(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        let cached = self.get_cached()?;
        let mut rc = cached.render_cache.lock().unwrap();

        match &urls {
            None => {
                if let Some(result) = &rc.css_no_urls {
                    return Ok(result.clone());
                }
                let ctx = cached.css_hbs_context.lock().unwrap();
                let result = render_css_with_hbs_context(&cached.shared, &ctx, &cached.css_context)
                    .map_err(to_napi_err)?;
                rc.css_no_urls = Some(result.clone());
                Ok(result)
            }
            Some(urls) => {
                // If the template doesn't reference {{src}}, URLs don't affect output
                if !cached.shared.css_template_uses_src {
                    drop(rc);
                    return self.generate_css(None);
                }
                if rc.css_last_urls.as_ref() == Some(urls) {
                    if let Some(result) = &rc.css_last_result {
                        return Ok(result.clone());
                    }
                }
                let src = make_src(&self.options, urls);
                let mut ctx = cached.css_hbs_context.lock().unwrap();
                let result =
                    render_css_with_src_mutate(&cached.shared, &mut ctx, &cached.css_context, &src)
                        .map_err(to_napi_err)?;
                rc.css_last_urls = Some(urls.clone());
                rc.css_last_result = Some(result.clone());
                Ok(result)
            }
        }
    }

    #[napi(ts_args_type = "urls?: Partial<Record<FontType, string>>")]
    pub fn generate_html(&self, urls: Option<HashMap<String, String>>) -> napi::Result<String> {
        let urls = urls.map(parse_native_urls).transpose()?;
        let cached = self.get_cached()?;
        let mut rc = cached.render_cache.lock().unwrap();

        match &urls {
            None => {
                if let Some(result) = &rc.html_no_urls {
                    return Ok(result.clone());
                }
                let ctx = cached.html_hbs_context.lock().unwrap();
                let result = render_html_with_hbs_context(
                    cached.html_registry.as_ref(),
                    &ctx,
                    &cached.html_context,
                )
                .map_err(to_napi_err)?;
                rc.html_no_urls = Some(result.clone());
                Ok(result)
            }
            Some(urls) => {
                // If the CSS template doesn't reference {{src}}, URLs don't affect output
                if !cached.shared.css_template_uses_src {
                    drop(rc);
                    return self.generate_html(None);
                }
                if rc.html_last_urls.as_ref() == Some(urls) {
                    if let Some(result) = &rc.html_last_result {
                        return Ok(result.clone());
                    }
                }
                // Render CSS with the custom URLs (in-place src mutate, no clone)
                let src = make_src(&self.options, urls);
                let styles = {
                    let mut css_ctx = cached.css_hbs_context.lock().unwrap();
                    render_css_with_src_mutate(
                        &cached.shared,
                        &mut css_ctx,
                        &cached.css_context,
                        &src,
                    )
                    .map_err(to_napi_err)?
                };
                // Hot path: default HTML template — inject styles directly, skip clone
                if self.options.html_template.is_none() {
                    let result = render_default_html_with_styles(&cached.html_context, &styles);
                    rc.html_last_urls = Some(urls.clone());
                    rc.html_last_result = Some(result.clone());
                    return Ok(result);
                }
                // Custom HTML template: in-place styles mutate, no clone
                let mut html_ctx = cached.html_hbs_context.lock().unwrap();
                let registry = cached
                    .html_registry
                    .as_ref()
                    .expect("HTML registry should exist for custom template");
                let result = crate::util::render_with_field_swap(
                    &mut html_ctx,
                    "styles",
                    serde_json::Value::String(styles),
                    |ctx| {
                        registry
                            .render_with_context("html", ctx)
                            .map_err(crate::util::to_io_err)
                    },
                )
                .map_err(to_napi_err)?;
                rc.html_last_urls = Some(urls.clone());
                rc.html_last_result = Some(result.clone());
                Ok(result)
            }
        }
    }
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
mod tests {
    use super::*;
    use crate::{finalize_generate_webfonts_options, resolve_generate_webfonts_options};

    fn build_result(template: Option<&str>) -> GenerateWebfontsResult {
        let fixture = crate::test_helpers::webfont_fixture("add.svg");

        let mut css_template = None;
        let cleanup_dir;
        if let Some(content) = template {
            let tmp = std::env::temp_dir().join(format!(
                "render-cache-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&tmp).unwrap();
            let path = tmp.join("template.hbs");
            std::fs::write(&path, content).unwrap();
            css_template = Some(path.to_string_lossy().into_owned());
            cleanup_dir = Some(tmp);
        } else {
            cleanup_dir = None;
        }

        let options = GenerateWebfontsOptions {
            css: Some(true),
            css_template,
            codepoints: Some(HashMap::from([("add".to_owned(), 0xE001u32)])),
            dest: "artifacts".to_owned(),
            files: vec![fixture],
            html: Some(false),
            font_name: Some("iconfont".to_owned()),
            ligature: Some(false),
            order: Some(vec![FontType::Svg]),
            start_codepoint: Some(0xE001),
            types: Some(vec![FontType::Svg]),
            ..Default::default()
        };

        let mut resolved = resolve_generate_webfonts_options(options).unwrap();
        let source_files: Vec<LoadedSvgFile> = resolved
            .files
            .iter()
            .map(|path| LoadedSvgFile {
                contents: std::fs::read_to_string(path).unwrap(),
                glyph_name: std::path::Path::new(path)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
                path: path.clone(),
            })
            .collect();
        finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();

        let result = GenerateWebfontsResult {
            cached: std::sync::OnceLock::new(),
            css_context: None,
            eot_font: None,
            html_context: None,
            options: resolved,
            source_files,
            svg_font: None,
            ttf_font: None,
            woff2_font: None,
            woff_font: None,
        };

        if let Some(dir) = cleanup_dir {
            // Don't clean up yet — template file needed for lazy compilation
            std::mem::forget(dir);
        }

        result
    }

    #[test]
    fn generate_css_returns_cached_result_on_repeated_calls_without_urls() {
        let result = build_result(None);

        let first = result.generate_css(None).unwrap();
        let second = result.generate_css(None).unwrap();

        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn generate_css_returns_cached_result_on_repeated_calls_with_same_urls() {
        let result = build_result(None);
        let urls = HashMap::from([("svg".to_owned(), "/a.svg".to_owned())]);

        let first = result.generate_css(Some(urls.clone())).unwrap();
        let second = result.generate_css(Some(urls)).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("/a.svg"));
    }

    #[test]
    fn generate_css_returns_different_result_for_different_urls() {
        let result = build_result(None);
        let urls_a = HashMap::from([("svg".to_owned(), "/a.svg".to_owned())]);
        let urls_b = HashMap::from([("svg".to_owned(), "/b.svg".to_owned())]);

        let result_a = result.generate_css(Some(urls_a)).unwrap();
        let result_b = result.generate_css(Some(urls_b)).unwrap();

        assert_ne!(result_a, result_b);
        assert!(result_a.contains("/a.svg"));
        assert!(result_b.contains("/b.svg"));
    }

    #[test]
    fn generate_css_cache_updates_when_urls_change() {
        let result = build_result(None);
        let urls_a = HashMap::from([("svg".to_owned(), "/a.svg".to_owned())]);
        let urls_b = HashMap::from([("svg".to_owned(), "/b.svg".to_owned())]);

        let first_a = result.generate_css(Some(urls_a.clone())).unwrap();
        let first_b = result.generate_css(Some(urls_b)).unwrap();
        let second_a = result.generate_css(Some(urls_a)).unwrap();

        assert_eq!(
            first_a, second_a,
            "returning to original urls should produce same result"
        );
        assert_ne!(first_a, first_b);
    }

    #[test]
    fn generate_css_cache_works_with_custom_template() {
        let result = build_result(Some("@font-face { src: {{{src}}}; }"));
        let urls = HashMap::from([("svg".to_owned(), "/cached.svg".to_owned())]);

        let first = result.generate_css(Some(urls.clone())).unwrap();
        let second = result.generate_css(Some(urls)).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("/cached.svg"));
    }

    #[test]
    fn generate_css_no_urls_and_with_urls_are_independent_caches() {
        let result = build_result(None);
        let urls = HashMap::from([("svg".to_owned(), "/custom.svg".to_owned())]);

        let no_urls = result.generate_css(None).unwrap();
        let with_urls = result.generate_css(Some(urls)).unwrap();
        let no_urls_again = result.generate_css(None).unwrap();

        assert_eq!(
            no_urls, no_urls_again,
            "no-urls cache should survive a with-urls call"
        );
        assert_ne!(no_urls, with_urls);
    }

    #[test]
    fn generate_css_with_urls_returns_no_urls_result_when_template_does_not_use_src() {
        let result = build_result(Some(".icon { font-family: {{fontName}}; }"));
        let urls = HashMap::from([("svg".to_owned(), "/should-not-appear.svg".to_owned())]);

        let no_urls = result.generate_css(None).unwrap();
        let with_urls = result.generate_css(Some(urls)).unwrap();

        assert_eq!(
            no_urls, with_urls,
            "template without {{src}} should ignore urls"
        );
        assert!(!with_urls.contains("/should-not-appear.svg"));
        assert!(
            with_urls.contains("iconfont"),
            "should still render the template"
        );
    }

    #[test]
    fn generate_html_with_urls_returns_no_urls_result_when_css_template_does_not_use_src() {
        let result = build_result(Some(".icon { font-family: {{fontName}}; }"));
        let urls = HashMap::from([("svg".to_owned(), "/should-not-appear.svg".to_owned())]);

        let no_urls = result.generate_html(None).unwrap();
        let with_urls = result.generate_html(Some(urls)).unwrap();

        assert_eq!(
            no_urls, with_urls,
            "CSS template without {{src}} means HTML is also unaffected by urls"
        );
    }

    #[test]
    fn generate_css_without_urls_produces_valid_css_using_css_fonts_url() {
        let result = build_result(None);

        let css = result.generate_css(None).unwrap();

        assert!(
            css.contains("@font-face"),
            "should contain @font-face declaration"
        );
        assert!(css.contains("font-family:"), "should contain font-family");
        assert!(
            css.contains("iconfont.svg?"),
            "should use font name in URL with hash"
        );
        assert!(
            css.contains("format(\"svg\")"),
            "should contain format declaration"
        );
        assert!(
            css.contains("content:"),
            "should contain codepoint content rules"
        );
    }

    #[test]
    fn generate_css_with_urls_replaces_default_urls_in_src() {
        let result = build_result(None);
        let urls = HashMap::from([("svg".to_owned(), "/cdn/icons.svg".to_owned())]);

        let css = result.generate_css(Some(urls)).unwrap();

        assert!(
            css.contains("/cdn/icons.svg"),
            "custom URL should appear in output"
        );
        assert!(
            !css.contains("iconfont.svg?"),
            "default hash-based URL should not appear"
        );
        assert!(
            css.contains("format(\"svg\")"),
            "format should still be present"
        );
    }

    #[test]
    fn generate_html_without_urls_produces_valid_html() {
        let result = build_result(None);

        let html = result.generate_html(None).unwrap();

        assert!(
            html.contains("<!DOCTYPE html>"),
            "should be a full HTML document"
        );
        assert!(html.contains("iconfont"), "should contain font name");
        assert!(html.contains("icon-add"), "should contain icon class name");
    }

    #[test]
    fn generate_html_with_urls_embeds_css_using_custom_urls() {
        let result = build_result(None);
        let urls = HashMap::from([("svg".to_owned(), "/cdn/icons.svg".to_owned())]);

        let html = result.generate_html(Some(urls)).unwrap();

        assert!(
            html.contains("/cdn/icons.svg"),
            "custom URL should appear in embedded CSS"
        );
        assert!(
            html.contains("icon-add"),
            "should still contain icon class name"
        );
    }

    #[test]
    fn generate_html_cache_returns_same_result_for_same_urls() {
        let result = build_result(None);
        let urls = HashMap::from([("svg".to_owned(), "/cached.svg".to_owned())]);

        let first = result.generate_html(Some(urls.clone())).unwrap();
        let second = result.generate_html(Some(urls)).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("/cached.svg"));
    }

    #[test]
    fn generate_html_cache_returns_different_result_for_different_urls() {
        let result = build_result(None);
        let urls_a = HashMap::from([("svg".to_owned(), "/a.svg".to_owned())]);
        let urls_b = HashMap::from([("svg".to_owned(), "/b.svg".to_owned())]);

        let result_a = result.generate_html(Some(urls_a)).unwrap();
        let result_b = result.generate_html(Some(urls_b)).unwrap();

        assert_ne!(result_a, result_b);
        assert!(result_a.contains("/a.svg"));
        assert!(result_b.contains("/b.svg"));
    }
}
