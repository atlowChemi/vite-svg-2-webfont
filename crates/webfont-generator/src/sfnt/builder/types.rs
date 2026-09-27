use std::sync::Arc;

use write_fonts::tables::glyf::{Bbox, SimpleGlyph};

pub(crate) struct TtfOptions<'a> {
    pub ascent: Option<f64>,
    pub copyright: Option<&'a str>,
    pub descent: Option<f64>,
    pub description: Option<&'a str>,
    pub font_height: Option<f64>,
    pub font_name: &'a str,
    pub font_style: Option<&'a str>,
    pub font_weight: Option<&'a str>,
    pub ligature: bool,
    pub manufacturer_url: Option<&'a str>,
    pub ts: Option<i64>,
    pub version: Option<&'a str>,
}

pub(super) struct CompiledGlyph {
    pub(super) advance_width: u16,
    pub(super) bbox: Bbox,
    pub(super) codepoint: u32,
    pub(super) outline: CompiledGlyphOutline,
    pub(super) left_side_bearing: i16,
    pub(super) name: String,
    pub(super) outline_key: Option<u64>,
    pub(super) source_index: usize,
}

pub(super) enum CompiledGlyphOutline {
    Inline(SimpleGlyph),
    Shared(Arc<CachedCompiledGlyph>),
}

impl CompiledGlyph {
    pub(super) fn simple_glyph(&self) -> &SimpleGlyph {
        match &self.outline {
            CompiledGlyphOutline::Inline(glyph) => glyph,
            CompiledGlyphOutline::Shared(glyph) => &glyph.simple_glyph,
        }
    }
}

pub(crate) struct CachedCompiledGlyph {
    pub(crate) advance_width: u16,
    pub(crate) bbox: Bbox,
    pub(crate) simple_glyph: SimpleGlyph,
}

pub(super) struct GlyphMetrics {
    pub(super) advance_width_max: u16,
    pub(super) bbox: (i16, i16, i16, i16),
    pub(super) max_contours: u16,
    pub(super) max_points: u16,
    pub(super) min_left_side_bearing: i16,
    pub(super) min_right_side_bearing: i32,
    pub(super) x_avg_char_width: i16,
    pub(super) x_max_extent: i32,
}

pub(super) type CmapAliases = Vec<(u32, usize)>;
