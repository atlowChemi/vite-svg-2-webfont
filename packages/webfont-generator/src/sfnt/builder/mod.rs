mod cache;
mod glyphs;
mod ligatures;
mod outlines;
mod tables;
#[cfg(test)]
mod tests;
mod types;

use std::io::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::input::ResolvedGenerateWebfontsOptions;
use crate::pipeline::TtfGlyphCache;
use crate::svg::types::ProcessedGlyph;

pub(crate) use types::{CachedCompiledGlyph, TtfOptions};

use glyphs::{
    build_glyf_table, compile_and_dedup_glyphs, compile_and_dedup_glyphs_cached,
    compute_glyph_metrics,
};
use ligatures::build_ligature_placeholders;
use tables::assemble_font;

pub(crate) fn ttf_options_from_options(
    options: &ResolvedGenerateWebfontsOptions,
) -> TtfOptions<'_> {
    let ttf_format = options
        .format_options
        .as_ref()
        .and_then(|value| value.ttf.as_ref());
    TtfOptions {
        ascent: options.ascent,
        copyright: ttf_format.and_then(|v| v.copyright.as_deref()),
        descent: options.descent,
        description: ttf_format.and_then(|v| v.description.as_deref()),
        font_height: options.font_height,
        font_name: &options.font_name,
        font_style: options.font_style.as_deref(),
        font_weight: options.font_weight.as_deref(),
        ligature: options.ligature,
        manufacturer_url: ttf_format.and_then(|v| v.url.as_deref()),
        ts: ttf_format.and_then(|v| v.ts),
        version: ttf_format.and_then(|v| v.version.as_deref()),
    }
}

pub(crate) fn build(
    options: TtfOptions,
    glyphs: &[ProcessedGlyph],
    cache: Option<&mut TtfGlyphCache>,
) -> Result<super::SerializedFontTables, Error> {
    match cache {
        Some(cache) => build_cached(options, glyphs, cache),
        None => build_uncached(options, glyphs),
    }
}

fn build_uncached(
    options: TtfOptions,
    glyphs: &[ProcessedGlyph],
) -> Result<super::SerializedFontTables, Error> {
    let font_height = options.font_height.unwrap_or_else(|| {
        glyphs
            .iter()
            .fold(0.0_f64, |current, glyph| current.max(glyph.height))
            .max(1.0)
    });
    let descent = options.descent.unwrap_or(0.0);
    let ascent = options.ascent.unwrap_or(font_height - descent);

    let (compiled_glyphs, cmap_aliases) = compile_and_dedup_glyphs(glyphs)?;
    let ligature_placeholders = build_ligature_placeholders(&compiled_glyphs, options.ligature);
    let (glyf, loca, loca_format) = build_glyf_table(&compiled_glyphs, &ligature_placeholders)?;
    let metrics = compute_glyph_metrics(&compiled_glyphs);

    assemble_font(
        &options,
        &compiled_glyphs,
        &cmap_aliases,
        &ligature_placeholders,
        glyf,
        loca,
        loca_format,
        &metrics,
        ascent,
        descent,
        font_height,
        None,
    )
}

fn build_cached(
    options: TtfOptions,
    glyphs: &[ProcessedGlyph],
    cache: &mut TtfGlyphCache,
) -> Result<super::SerializedFontTables, Error> {
    let font_height = options.font_height.unwrap_or_else(|| {
        glyphs
            .iter()
            .fold(0.0_f64, |current, glyph| current.max(glyph.height))
            .max(1.0)
    });
    let descent = options.descent.unwrap_or(0.0);
    let ascent = options.ascent.unwrap_or(font_height - descent);

    let (compiled_glyphs, cmap_aliases) = compile_and_dedup_glyphs_cached(glyphs, cache)?;
    let ligature_placeholders = build_ligature_placeholders(&compiled_glyphs, options.ligature);
    let (glyf, loca, loca_format) = build_glyf_table(&compiled_glyphs, &ligature_placeholders)?;
    let metrics = compute_glyph_metrics(&compiled_glyphs);

    assemble_font(
        &options,
        &compiled_glyphs,
        &cmap_aliases,
        &ligature_placeholders,
        glyf,
        loca,
        loca_format,
        &metrics,
        ascent,
        descent,
        font_height,
        Some(cache),
    )
}

pub(super) fn clamp_to_i16(value: f64) -> i16 {
    value
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX))
        .round() as i16
}
pub(super) fn clamp_to_u16(value: f64, min_value: u16, max_value: u16) -> u16 {
    value
        .clamp(f64::from(min_value), f64::from(max_value))
        .round() as u16
}
pub(super) fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
