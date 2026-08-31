use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::io::Error;
use std::sync::Arc;

use write_fonts::tables::glyf::{GlyfLocaBuilder, Glyph, SimpleGlyph};

use crate::pipeline::TtfGlyphCache;
use crate::svg::types::ProcessedGlyph;

use super::cache::compiled_glyph_cache_key;
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
    Ok(builder.build())
}

pub(super) fn compute_glyph_metrics(glyphs: &[CompiledGlyph]) -> GlyphMetrics {
    let bbox = glyphs.iter().fold(
        (0_i16, 0_i16, 0_i16, 0_i16),
        |(x_min, y_min, x_max, y_max), g| {
            (
                min(x_min, g.bbox.x_min),
                min(y_min, g.bbox.y_min),
                max(x_max, g.bbox.x_max),
                max(y_max, g.bbox.y_max),
            )
        },
    );
    GlyphMetrics {
        advance_width_max: glyphs.iter().map(|g| g.advance_width).max().unwrap_or(0),
        bbox,
        max_contours: glyphs
            .iter()
            .map(|g| g.simple_glyph().contours.len() as u16)
            .max()
            .unwrap_or(0),
        max_points: glyphs
            .iter()
            .map(|g| {
                g.simple_glyph()
                    .contours
                    .iter()
                    .map(|c| c.len())
                    .sum::<usize>() as u16
            })
            .max()
            .unwrap_or(0),
        min_left_side_bearing: glyphs
            .iter()
            .map(|g| g.left_side_bearing)
            .min()
            .unwrap_or(0),
        min_right_side_bearing: glyphs
            .iter()
            .map(|g| {
                i32::from(g.advance_width)
                    - (i32::from(g.left_side_bearing) + i32::from(g.bbox.x_max)
                        - i32::from(g.bbox.x_min))
            })
            .min()
            .unwrap_or(0),
        x_avg_char_width: average_advance_width(glyphs),
        x_max_extent: glyphs
            .iter()
            .map(|g| {
                i32::from(g.left_side_bearing) + (i32::from(g.bbox.x_max) - i32::from(g.bbox.x_min))
            })
            .max()
            .unwrap_or(0),
    }
}

fn compile_glyph(source_index: usize, glyph: &ProcessedGlyph) -> Result<CompiledGlyph, Error> {
    let advance_width = clamp_to_u16(glyph.width.round(), 0, u16::MAX);
    let simple_glyph = compile_simple_glyph(glyph)?;
    let bbox = simple_glyph.bbox;
    Ok(CompiledGlyph {
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

fn compile_simple_glyph(glyph: &ProcessedGlyph) -> Result<SimpleGlyph, Error> {
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
    glyph.ttf_path_hash.unwrap_or(glyph.path_data.len() as u64)
}

fn glyph_paths_equal(left: &ProcessedGlyph, right: &ProcessedGlyph) -> bool {
    match (&left.ttf_path, &right.ttf_path) {
        (Some(left), Some(right)) => left.elements() == right.elements(),
        _ => left.path_data == right.path_data,
    }
}

fn average_advance_width(compiled_glyphs: &[CompiledGlyph]) -> i16 {
    let non_zero_widths = compiled_glyphs
        .iter()
        .map(|glyph| glyph.advance_width)
        .filter(|width| *width > 0)
        .collect::<Vec<_>>();
    if non_zero_widths.is_empty() {
        return 0;
    }
    let total: u32 = non_zero_widths.iter().map(|width| u32::from(*width)).sum();
    clamp_to_i16((total / non_zero_widths.len() as u32) as f64)
}
