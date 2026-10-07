use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::io::Error;
use std::sync::Arc;

use write_fonts::tables::glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph};

use crate::pipeline::TtfGlyphCache;
use crate::svg::types::ProcessedGlyph;
use crate::types::color::ResolvedLayerPaint;

use super::cache::{compiled_glyph_cache_key, table_cache_key};
use super::ligatures::LigaturePlaceholderGlyph;
use super::outlines::{quadratic_path, quadratic_path_from_svg_path_data};
use super::types::{
    CachedCompiledGlyph, CmapAliases, CompiledGlyph, CompiledGlyphOutline, GlyphMetrics,
};
use super::{clamp_to_i16, clamp_to_u16};

pub(super) fn compile_and_dedup_glyphs(
    glyphs: &[ProcessedGlyph],
) -> Result<(Vec<CompiledGlyph>, CmapAliases), Error> {
    let mut compiled: Vec<CompiledGlyph> = Vec::with_capacity(glyphs.len());
    let mut aliases: Vec<(u32, usize)> = Vec::new();
    let mut seen: HashMap<(u64, u16), Vec<usize>> = HashMap::new();
    for (i, glyph) in glyphs.iter().enumerate() {
        let advance_width = clamp_to_u16(glyph.width.round(), 0, u16::MAX);
        let key = (glyph_path_bucket(glyph), advance_width);
        let duplicate_of = seen.get(&key).and_then(|indices| {
            indices
                .iter()
                .find(|&&idx| glyph_paths_equal(&glyphs[compiled[idx].source_index], glyph))
                .copied()
        });
        if let Some(first_idx) = duplicate_of {
            aliases.push((glyph.codepoint, first_idx));
        } else {
            let idx = compiled.len();
            seen.entry(key).or_default().push(idx);
            compiled.push(compile_glyph(i, glyph)?);
        }
    }
    Ok((compiled, aliases))
}

pub(super) fn compile_and_dedup_glyphs_cached(
    glyphs: &[ProcessedGlyph],
    cache: &mut TtfGlyphCache,
) -> Result<(Vec<CompiledGlyph>, CmapAliases), Error> {
    let mut compiled: Vec<CompiledGlyph> = Vec::with_capacity(glyphs.len());
    let mut aliases: Vec<(u32, usize)> = Vec::new();
    let mut seen: HashMap<(u64, u16), Vec<usize>> = HashMap::new();
    let mut used_keys = HashSet::with_capacity(glyphs.len());
    for (i, glyph) in glyphs.iter().enumerate() {
        let advance_width = clamp_to_u16(glyph.width.round(), 0, u16::MAX);
        let key = (glyph_path_bucket(glyph), advance_width);
        let duplicate_of = seen.get(&key).and_then(|indices| {
            indices
                .iter()
                .find(|&&idx| glyph_paths_equal(&glyphs[compiled[idx].source_index], glyph))
                .copied()
        });
        if let Some(first_idx) = duplicate_of {
            aliases.push((glyph.codepoint, first_idx));
        } else {
            let cache_key = compiled_glyph_cache_key(glyph, advance_width);
            let cached = match cache.entries.get(&cache_key) {
                Some(cached) => Arc::clone(cached),
                None => {
                    #[cfg(test)]
                    {
                        cache.compile_count += 1;
                    }
                    let simple_glyph = compile_simple_glyph(glyph)?;
                    let bbox = simple_glyph.bbox;
                    let cached = Arc::new(CachedCompiledGlyph {
                        advance_width,
                        bbox,
                        simple_glyph,
                    });
                    cache.entries.insert(cache_key, Arc::clone(&cached));
                    cached
                }
            };
            used_keys.insert(cache_key);
            let idx = compiled.len();
            seen.entry(key).or_default().push(idx);
            compiled.push(CompiledGlyph {
                color_layers: glyph.color_layers.clone(),
                advance_width: cached.advance_width,
                bbox: cached.bbox,
                codepoint: glyph.codepoint,
                left_side_bearing: cached.bbox.x_min,
                name: glyph.name.clone(),
                outline_key: Some(cache_key),
                outline: CompiledGlyphOutline::Shared(cached),
                source_index: i,
            });
        }
    }
    cache.entries.retain(|key, _| used_keys.contains(key));
    Ok((compiled, aliases))
}

pub(super) fn build_glyf_table(
    compiled_glyphs: &[CompiledGlyph],
    ligature_placeholders: &[LigaturePlaceholderGlyph],
    layers: &[CompiledGlyph],
) -> Result<
    (
        write_fonts::tables::glyf::Glyf,
        write_fonts::tables::loca::Loca,
        write_fonts::tables::loca::LocaFormat,
    ),
    Error,
> {
    let mut builder = GlyfLocaBuilder::new();
    builder
        .add_glyph(&Glyph::Empty)
        .map_err(|error| Error::other(format!("Failed to add .notdef glyph: {error}")))?;
    for glyph in compiled_glyphs {
        builder.add_glyph(glyph.simple_glyph()).map_err(|error| {
            Error::other(format!("Failed to compile glyph '{}': {error}", glyph.name))
        })?;
    }
    for placeholder in ligature_placeholders {
        builder.add_glyph(&Glyph::Empty).map_err(|error| {
            Error::other(format!(
                "Failed to add ligature placeholder '{}': {error}",
                placeholder.name
            ))
        })?;
    }
    for layer in layers {
        builder.add_glyph(layer.simple_glyph()).map_err(|error| {
            Error::other(format!("Failed to compile layer '{}': {error}", layer.name))
        })?;
    }
    Ok(builder.build())
}

pub(super) fn compute_glyph_metrics<'a>(
    glyphs: impl Iterator<Item = &'a CompiledGlyph>,
) -> GlyphMetrics {
    let mut metrics = GlyphMetrics {
        advance_width_max: 0,
        bbox: (0, 0, 0, 0),
        max_contours: 0,
        max_points: 0,
        min_left_side_bearing: i16::MAX,
        min_right_side_bearing: i32::MAX,
        x_avg_char_width: 0,
        x_max_extent: i32::MIN,
    };
    let mut any = false;
    let mut total_width = 0_u64;
    let mut width_count = 0_u64;
    for glyph in glyphs {
        any = true;
        let contours = &glyph.simple_glyph().contours;
        let extent = i32::from(glyph.left_side_bearing) + i32::from(glyph.bbox.x_max)
            - i32::from(glyph.bbox.x_min);
        metrics.bbox.0 = min(metrics.bbox.0, glyph.bbox.x_min);
        metrics.bbox.1 = min(metrics.bbox.1, glyph.bbox.y_min);
        metrics.bbox.2 = max(metrics.bbox.2, glyph.bbox.x_max);
        metrics.bbox.3 = max(metrics.bbox.3, glyph.bbox.y_max);
        metrics.advance_width_max = max(metrics.advance_width_max, glyph.advance_width);
        metrics.max_contours = max(metrics.max_contours, contours.len() as u16);
        metrics.max_points = max(
            metrics.max_points,
            contours.iter().map(|c| c.len()).sum::<usize>() as u16,
        );
        metrics.min_left_side_bearing = min(metrics.min_left_side_bearing, glyph.left_side_bearing);
        metrics.min_right_side_bearing = min(
            metrics.min_right_side_bearing,
            i32::from(glyph.advance_width) - extent,
        );
        metrics.x_max_extent = max(metrics.x_max_extent, extent);
        if glyph.advance_width != 0 {
            total_width += u64::from(glyph.advance_width);
            width_count += 1;
        }
    }
    if !any {
        metrics.min_left_side_bearing = 0;
        metrics.min_right_side_bearing = 0;
        metrics.x_max_extent = 0;
    }
    metrics.x_avg_char_width =
        clamp_to_i16(total_width.checked_div(width_count).unwrap_or(0) as f64);
    metrics
}

fn compile_glyph(source_index: usize, glyph: &ProcessedGlyph) -> Result<CompiledGlyph, Error> {
    let advance_width = clamp_to_u16(glyph.width.round(), 0, u16::MAX);
    let simple_glyph = compile_simple_glyph(glyph)?;
    let bbox = simple_glyph.bbox;
    Ok(CompiledGlyph {
        color_layers: glyph.color_layers.clone(),
        advance_width,
        bbox,
        codepoint: glyph.codepoint,
        left_side_bearing: bbox.x_min,
        name: glyph.name.clone(),
        outline: CompiledGlyphOutline::Inline(simple_glyph),
        outline_key: None,
        source_index,
    })
}

pub(super) fn compile_simple_glyph(glyph: &ProcessedGlyph) -> Result<SimpleGlyph, Error> {
    let path = match &glyph.ttf_path {
        Some(path) => quadratic_path(path)?,
        None => quadratic_path_from_svg_path_data(&glyph.path_data)?,
    };
    SimpleGlyph::from_bezpath(&path).map_err(|error| {
        Error::other(format!(
            "Failed to convert glyph '{}' into a TrueType outline: {error:?}",
            glyph.name
        ))
    })
}

fn glyph_path_bucket(glyph: &ProcessedGlyph) -> u64 {
    let fallback = glyph.ttf_path_hash.unwrap_or(glyph.path_data.len() as u64);
    let Some(layers) = &glyph.color_layers else {
        return fallback;
    };
    table_cache_key(b"COLR", |hasher| {
        hasher.write_u64(fallback);
        hasher.write_usize(layers.len());
        for layer in layers.iter() {
            hasher.write_u64(layer.outline_hash);
            match layer.paint {
                ResolvedLayerPaint::Foreground { alpha } => {
                    hasher.write_u8(0);
                    hasher.write_u32(if alpha == 0.0 { 0 } else { alpha.to_bits() });
                }
                ResolvedLayerPaint::Solid {
                    red,
                    green,
                    blue,
                    alpha,
                } => {
                    hasher.write(&[1, red, green, blue]);
                    hasher.write_u32(if alpha == 0.0 { 0 } else { alpha.to_bits() });
                }
            }
        }
    })
}

fn glyph_paths_equal(left: &ProcessedGlyph, right: &ProcessedGlyph) -> bool {
    left.color_layers == right.color_layers
        && match (&left.ttf_path, &right.ttf_path) {
            (Some(left), Some(right)) => left.elements() == right.elements(),
            _ => left.path_data == right.path_data,
        }
}
