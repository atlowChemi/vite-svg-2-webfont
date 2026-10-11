use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use kurbo::BezPath;
use usvg::tiny_skia_path::Path as TinyPath;

use crate::input::LoadedSvgSource;
use crate::types::color::{ProcessedColorLayer, ResolvedColorSelection, ResolvedLayerPaint};

pub(crate) struct SvgOptions<'a> {
    pub color_selection: Option<&'a ResolvedColorSelection>,
    pub ascent: Option<f64>,
    pub center_horizontally: Option<bool>,
    pub center_vertically: Option<bool>,
    pub codepoints: &'a BTreeMap<String, u32>,
    pub descent: Option<f64>,
    pub fixed_width: Option<bool>,
    pub font_height: Option<f64>,
    pub font_id: Option<&'a str>,
    pub font_name: &'a str,
    pub font_style: Option<&'a str>,
    pub font_weight: Option<&'a str>,
    pub ligature: bool,
    pub metadata: Option<&'a str>,
    pub normalize: bool,
    pub optimize_output: Option<bool>,
    pub preserve_aspect_ratio: Option<bool>,
    pub round: Option<f64>,
    pub serialize_path: bool,
    pub structure_path: bool,
}

impl SvgOptions<'_> {
    pub fn selects_color(&self, name: &str) -> bool {
        self.color_selection
            .is_some_and(|selection| selection.contains(name))
    }
}

#[derive(Debug)]
pub(crate) struct ParsedColorLayer {
    pub path_index: usize,
    pub paint: ResolvedLayerPaint,
    pub fill_rule: usvg::FillRule,
}

#[derive(Clone)]
pub(crate) struct ParsedGlyph {
    pub color_layers: Option<Arc<[ParsedColorLayer]>>,
    pub codepoint: u32,
    pub height: f64,
    pub index: usize,
    pub name: String,
    pub paths: Vec<TinyPath>,
    pub width: f64,
}

pub(crate) struct GlyphWorkItem<'a> {
    pub codepoint: u32,
    pub index: usize,
    pub name: &'a str,
    pub source_file: &'a LoadedSvgSource,
}

#[derive(Clone)]
pub(crate) struct ProcessedGlyph {
    pub color_layers: Option<Arc<[ProcessedColorLayer]>>,
    pub codepoint: u32,
    pub height: f64,
    pub index: usize,
    pub name: String,
    pub path_data: Arc<str>,
    pub ttf_path: Option<Arc<BezPath>>,
    pub ttf_path_hash: Option<u64>,
    pub width: f64,
}

#[derive(Clone)]
pub(crate) struct CachedProcessedGlyph {
    pub color_layers: Option<Arc<[ProcessedColorLayer]>>,
    pub height: f64,
    pub path_data: Arc<str>,
    pub ttf_path: Option<Arc<BezPath>>,
    pub ttf_path_hash: Option<u64>,
    pub width: f64,
}

#[derive(Clone)]
pub(crate) struct PreparedSvgFont {
    pub ascent: f64,
    pub descent: f64,
    pub font_height: f64,
    pub font_id: String,
    pub font_width: f64,
    pub metadata: String,
    pub processed_glyphs: Vec<ProcessedGlyph>,
}

pub(crate) struct PreparedVariantFamily {
    pub ascent: f64,
    pub descent: f64,
    pub font_height: f64,
    pub glyphs: Vec<ProcessedVariantGlyph>,
}

pub(crate) struct ProcessedVariantGlyph {
    pub name: String,
    pub codepoint: u32,
    pub advance_width: f64,
    pub outlines: Box<[Option<ProcessedGlyph>]>,
}

/// The content-derived geometry of one parsed glyph (everything in [`ParsedGlyph`] except the
/// assigned `codepoint`/`index`/`name`, which are reassigned on every build). Cached so an
/// incremental rebuild can reuse a glyph whose SVG source didn't change.
pub(crate) struct CachedGlyph {
    pub color_layers: Option<Arc<[ParsedColorLayer]>>,
    pub height: f64,
    pub paths: Vec<TinyPath>,
    pub width: f64,
}

/// Parsed-glyph cache keyed by opaque source ID. A present entry is treated as up to date:
/// the caller evicts IDs whose content changed, so unchanged sources need no re-hashing.
/// File adapters use verbatim input paths as IDs; the cache never interprets them as paths.
#[derive(Clone, Default)]
pub(crate) struct GlyphCache {
    /// Parsed geometry keyed by ID for the current/last known source set.
    pub entries: HashMap<String, Arc<CachedGlyph>>,
    /// Last seen source hash per ID, used to ignore no-op updates.
    pub content_hashes: HashMap<String, [u8; 16]>,
    /// Most recently parsed mode per source hash. Reuse requires matching color-layer presence;
    /// per-ID entries retain both modes when identical sources have different selections.
    pub by_content_hash: HashMap<[u8; 16], Arc<CachedGlyph>>,
    /// Processed path data keyed by ID for unchanged sources while metrics/options stay stable.
    pub processed_entries: HashMap<String, CachedProcessedGlyph>,
    pub processed_signature: Option<[u8; 16]>,
    #[cfg(test)]
    pub parse_count: usize,
    #[cfg(test)]
    pub process_count: usize,
}
