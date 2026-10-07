use std::collections::HashMap;
use std::hash::Hasher;
use std::io::{Error, ErrorKind};

use write_fonts::tables::colr::{
    BaseGlyphList, BaseGlyphPaint, Colr, LayerList, Paint, PaintColrLayers, PaintGlyph, PaintSolid,
};
use write_fonts::tables::cpal::{ColorRecord, Cpal};
use write_fonts::tables::glyf::SimpleGlyph;
use write_fonts::types::{F2Dot14, GlyphId16};

use super::cache::{dump_ttf_table, table_cache_key};
use super::outlines::quadratic_path;
use super::types::{CompiledGlyph, CompiledGlyphOutline};
use crate::types::color::ResolvedLayerPaint;

#[derive(Default)]
pub(super) struct ColorFont {
    pub layers: Vec<CompiledGlyph>,
    pub tables: Vec<([u8; 4], Vec<u8>)>,
}

pub(super) fn build_color(
    glyphs: &[CompiledGlyph],
    placeholder_count: usize,
) -> Result<ColorFont, Error> {
    let selectable_count = glyphs
        .len()
        .checked_add(placeholder_count)
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Font glyph count overflow."))?;
    let count = glyphs.iter().try_fold(selectable_count, |count, glyph| {
        count
            .checked_add(glyph.color_layers.as_ref().map_or(0, |layers| layers.len()))
            .filter(|count| *count <= usize::from(u16::MAX))
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Color layers for glyph '{}' exceed the 65,535-glyph limit.",
                        glyph.name
                    ),
                )
            })
    })?;
    if count > usize::from(u16::MAX) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Font exceeds the 65,535-glyph limit.",
        ));
    }
    let mut result = ColorFont::default();
    let mut palette = Vec::new();
    let mut palette_indices = HashMap::new();
    let mut paints = Vec::new();
    let mut bases = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        let Some(layers) = &glyph.color_layers else {
            continue;
        };
        if layers.is_empty() {
            continue;
        }
        let mut leaves = Vec::with_capacity(layers.len());
        for layer in layers.iter() {
            let gid = (selectable_count + result.layers.len()) as u16;
            let (palette_index, alpha) = match layer.paint {
                ResolvedLayerPaint::Foreground { alpha } => (u16::MAX, alpha),
                ResolvedLayerPaint::Solid {
                    red,
                    green,
                    blue,
                    alpha,
                } => {
                    let key = (red, green, blue);
                    let palette_index = match palette_indices.get(&key) {
                        Some(index) => *index,
                        None => {
                            if palette.len() >= usize::from(u16::MAX) {
                                return Err(Error::new(
                                    ErrorKind::InvalidInput,
                                    "CPAL exceeds 65,535 fixed colors.",
                                ));
                            }
                            let index = palette.len() as u16;
                            palette.push(ColorRecord::new(blue, green, red, 255));
                            palette_indices.insert(key, index);
                            index
                        }
                    };
                    (palette_index, alpha)
                }
            };
            let outline =
                SimpleGlyph::from_bezpath(&quadratic_path(&layer.outline)?).map_err(|error| {
                    Error::other(format!(
                        "Failed to compile color layer for '{}': {error:?}",
                        glyph.name
                    ))
                })?;
            result.layers.push(CompiledGlyph {
                color_layers: None,
                advance_width: 0,
                bbox: outline.bbox,
                codepoint: u32::MAX,
                left_side_bearing: outline.bbox.x_min,
                name: format!("colr.layer{gid}"),
                outline_key: Some(table_cache_key(b"COLR", |hasher| {
                    hasher.write_u64(layer.outline_hash)
                })),
                outline: CompiledGlyphOutline::Inline(outline),
                source_index: glyph.source_index,
            });
            leaves.push(
                PaintGlyph::new(
                    PaintSolid::new(palette_index, F2Dot14::from_f32(alpha)).into(),
                    GlyphId16::new(gid),
                )
                .into(),
            );
        }
        let root = append_layers(leaves, &mut paints)?;
        bases.push(BaseGlyphPaint::new(
            GlyphId16::new((index + 1) as u16),
            root,
        ));
    }
    if bases.is_empty() {
        return Ok(result);
    }
    let cpal = build_palette(palette)?;
    let colr = Colr {
        base_glyph_list: Some(BaseGlyphList::new(bases.len() as u32, bases)).into(),
        layer_list: Some(LayerList::new(paints.len() as u32, paints)).into(),
        ..Default::default()
    };
    result.tables.push(dump_ttf_table(&colr, "COLR")?);
    result.tables.push(dump_ttf_table(&cpal, "CPAL")?);
    Ok(result)
}

// Group at most 255 children per node, retaining the source paint order.
pub(super) fn append_layers(
    mut children: Vec<Paint>,
    paints: &mut Vec<Paint>,
) -> Result<Paint, Error> {
    loop {
        let first = u32::try_from(paints.len()).map_err(Error::other)?;
        let count = children.len();
        paints
            .len()
            .checked_add(count)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "COLR LayerList count overflow."))?;
        paints.append(&mut children);
        if count <= 255 {
            return Ok(PaintColrLayers::new(count as u8, first).into());
        }
        children = (0..count)
            .step_by(255)
            .map(|offset| {
                PaintColrLayers::new((count - offset).min(255) as u8, first + offset as u32).into()
            })
            .collect();
    }
}

pub(super) fn build_palette(mut colors: Vec<ColorRecord>) -> Result<Cpal, Error> {
    if colors.is_empty() {
        colors.push(ColorRecord::new(0, 0, 0, 255));
    }
    let count = u16::try_from(colors.len())
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "CPAL exceeds 65,535 fixed colors."))?;
    Ok(Cpal::new(count, 1, count, Some(colors), vec![0]))
}
