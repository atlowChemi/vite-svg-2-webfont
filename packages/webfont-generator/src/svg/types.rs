use std::collections::BTreeMap;

use usvg::tiny_skia_path::Path as TinyPath;

use crate::types::LoadedSvgFile;

pub(crate) struct SvgOptions<'a> {
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
}

pub(crate) struct ParsedGlyph {
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
    pub source_file: &'a LoadedSvgFile,
}

pub(crate) struct ProcessedGlyph {
    pub codepoint: u32,
    pub height: f64,
    pub index: usize,
    pub name: String,
    pub path_data: String,
    pub unicode_values: Vec<String>,
    pub width: f64,
}

pub(crate) struct PreparedSvgFont {
    pub ascent: f64,
    pub descent: f64,
    pub font_height: f64,
    pub font_id: String,
    pub font_width: f64,
    pub metadata: String,
    pub processed_glyphs: Vec<ProcessedGlyph>,
}
