use std::io::{Error, ErrorKind};

use write_fonts::tables::cmap::Cmap;
use write_fonts::tables::fvar::{AxisInstanceArrays, Fvar, InstanceRecord, VariationAxisRecord};
use write_fonts::tables::glyf::SimpleGlyph;
use write_fonts::tables::stat::{AxisRecord, AxisValue, AxisValueTableFlags, Stat};
use write_fonts::types::{Fixed, GlyphId, GlyphId16, NameId, Tag};

use crate::input::ResolvedVariants;
use crate::sfnt::SerializedFontTables;
use crate::svg::types::PreparedVariantFamily;

use super::clamp_to_u16;
use super::glyphs::{build_glyf_table, compile_simple_glyph, compute_glyph_metrics};
use super::tables::{
    assemble_font, build_name_table, build_os2, derive_version_string, make_windows_name_record,
};
use super::types::{CompiledGlyph, CompiledGlyphOutline, TtfOptions};

const WEIGHT_NAME_ID: u16 = 256;
const FIRST_VARIANT_NAME_ID: u16 = 257;

pub(crate) struct VariantFontBuild {
    pub(crate) tables: SerializedFontTables,
    pub(crate) presentation_gids: Vec<Box<[GlyphId16]>>,
}

pub(crate) fn build_variant(
    options: TtfOptions<'_>,
    family: &PreparedVariantFamily,
    variants: &ResolvedVariants,
) -> Result<VariantFontBuild, Error> {
    if variants.variants.is_empty() || variants.default_index >= variants.variants.len() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid resolved variants.",
        ));
    }
    if family
        .glyphs
        .iter()
        .any(|glyph| glyph.outlines.len() != variants.variants.len())
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Variant glyph matrix does not match resolved variants.",
        ));
    }

    let mut physical = Vec::new();
    let mut matrix = vec![vec![0_usize; variants.variants.len()]; family.glyphs.len()];
    for logical_index in 0..family.glyphs.len() {
        add_presentation(
            family,
            logical_index,
            variants.default_index,
            true,
            &mut physical,
            &mut matrix,
        )?;
    }
    for logical_index in 0..family.glyphs.len() {
        for variant_index in 0..variants.variants.len() {
            if variant_index != variants.default_index {
                add_presentation(
                    family,
                    logical_index,
                    variant_index,
                    false,
                    &mut physical,
                    &mut matrix,
                )?;
            }
        }
    }
    let presentation_gids = checked_gids(physical.len(), matrix)?;

    let (glyf, loca, loca_format) = build_glyf_table(&physical, &[])?;
    let metrics = compute_glyph_metrics(&physical);
    let default_weight = variants.variants[variants.default_index].weight.to_string();
    let base_options = TtfOptions {
        ascent: Some(family.ascent),
        copyright: options.copyright,
        descent: Some(family.descent),
        description: options.description,
        font_height: Some(family.font_height),
        font_name: options.font_name,
        font_style: options.font_style,
        font_weight: Some(&default_weight),
        ligature: false,
        manufacturer_url: options.manufacturer_url,
        ts: options.ts,
        version: options.version,
    };
    let base = assemble_font(
        &base_options,
        &physical,
        &[],
        &[],
        glyf,
        loca,
        loca_format,
        &metrics,
        family.ascent,
        family.descent,
        family.font_height,
        None,
    )?;

    let cmap = Cmap::from_mappings(family.glyphs.iter().enumerate().filter_map(
        |(logical_index, glyph)| {
            char::from_u32(glyph.codepoint).map(|codepoint| {
                (
                    codepoint,
                    GlyphId::new(u32::from(
                        presentation_gids[logical_index][variants.default_index].to_u16(),
                    )),
                )
            })
        },
    ))
    .map_err(|error| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Failed to build cmap table: {error}"),
        )
    })?;

    let default_name = &variants.variants[variants.default_index].name;
    let mut name = build_name_table(
        options.font_name,
        default_name,
        Some(&variant_postscript_name(options.font_name, default_name)),
        options.copyright,
        options.description,
        options.manufacturer_url,
        derive_version_string(options.version).as_deref(),
    );
    name.name_record
        .push(make_windows_name_record(WEIGHT_NAME_ID, "Weight"));
    name.name_record.extend(
        variants
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                make_windows_name_record(FIRST_VARIANT_NAME_ID + index as u16, &variant.name)
            }),
    );
    name.name_record.sort();

    let name_id = |index: usize| NameId::new(FIRST_VARIANT_NAME_ID + index as u16);
    let fvar = Fvar::new(AxisInstanceArrays::new(
        vec![VariationAxisRecord::new(
            Tag::new(b"wght"),
            Fixed::from_i32(i32::from(variants.variants[0].weight)),
            Fixed::from_i32(i32::from(variants.variants[variants.default_index].weight)),
            Fixed::from_i32(i32::from(variants.variants.last().unwrap().weight)),
            0,
            NameId::new(WEIGHT_NAME_ID),
        )],
        variants
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| InstanceRecord {
                subfamily_name_id: name_id(index),
                coordinates: vec![Fixed::from_i32(i32::from(variant.weight))],
                ..Default::default()
            })
            .collect(),
    ));
    let stat = Stat::new(
        vec![AxisRecord::new(
            Tag::new(b"wght"),
            NameId::new(WEIGHT_NAME_ID),
            0,
        )],
        variants
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                AxisValue::format_1(
                    0,
                    if index == variants.default_index {
                        AxisValueTableFlags::ELIDABLE_AXIS_VALUE_NAME
                    } else {
                        AxisValueTableFlags::empty()
                    },
                    name_id(index),
                    Fixed::from_i32(i32::from(variant.weight)),
                )
            })
            .collect(),
        name_id(variants.default_index),
    );

    let mut tables = base
        .tables()
        .iter()
        .filter(|table| table.tag != *b"OS/2" && table.tag != *b"cmap" && table.tag != *b"name")
        .map(|table| (table.tag, table.bytes.clone()))
        .collect::<Vec<_>>();
    tables.push((
        *b"cmap",
        write_fonts::dump_table(&cmap).map_err(Error::other)?,
    ));
    tables.push((
        *b"name",
        write_fonts::dump_table(&name).map_err(Error::other)?,
    ));
    tables.push((
        *b"OS/2",
        write_fonts::dump_table(&build_os2(
            &base_options,
            &metrics,
            family.ascent,
            family.descent,
            family.glyphs.iter().map(|glyph| glyph.codepoint),
        ))
        .map_err(Error::other)?,
    ));
    tables.push((
        *b"fvar",
        write_fonts::dump_table(&fvar).map_err(Error::other)?,
    ));
    tables.push((
        *b"STAT",
        write_fonts::dump_table(&stat).map_err(Error::other)?,
    ));

    Ok(VariantFontBuild {
        tables: SerializedFontTables::new(tables)?,
        presentation_gids,
    })
}

fn add_presentation(
    family: &PreparedVariantFamily,
    logical_index: usize,
    variant_index: usize,
    is_default: bool,
    physical: &mut Vec<CompiledGlyph>,
    matrix: &mut [Vec<usize>],
) -> Result<(), Error> {
    let logical = &family.glyphs[logical_index];
    let advance_width = clamp_to_u16(logical.advance_width.round(), 0, u16::MAX);
    let outline = match &logical.outlines[variant_index] {
        Some(glyph) => compile_simple_glyph(glyph)?,
        None => SimpleGlyph::default(),
    };
    // ponytail: exact linear scan; add outline-hash buckets if large variant families make this hot.
    let physical_index = physical
        .iter()
        .position(|glyph| {
            (!is_default || glyph.source_index == logical_index)
                && glyph.advance_width == advance_width
                && glyph.simple_glyph() == &outline
        })
        .unwrap_or_else(|| {
            let index = physical.len();
            let bbox = outline.bbox;
            physical.push(CompiledGlyph {
                advance_width,
                bbox,
                // Temporary unique values keep the shared static assembler's cmap valid; the
                // variant cmap below replaces it before these tables are returned.
                codepoint: index as u32,
                left_side_bearing: bbox.x_min,
                name: presentation_name(logical.name.as_str(), variant_index, is_default),
                outline: CompiledGlyphOutline::Inline(outline),
                outline_key: None,
                source_index: logical_index,
            });
            index
        });
    matrix[logical_index][variant_index] = physical_index + 1;
    Ok(())
}

fn presentation_name(logical_name: &str, variant_index: usize, is_default: bool) -> String {
    if is_default {
        logical_name.to_owned()
    } else {
        format!("{logical_name}.{variant_index}")
    }
}

fn variant_postscript_name(font_family: &str, variant_name: &str) -> String {
    const HASH_LEN: usize = 32;
    const MAX_LEN: usize = 63;

    let identity = format!("{font_family}\0{variant_name}");
    let hash = format!("{:x}", md5::compute(identity.as_bytes()));
    let mut prefix = String::new();
    for character in format!("{font_family}-{variant_name}").chars() {
        let valid = character.is_ascii_graphic()
            && !matches!(
                character,
                '[' | ']' | '(' | ')' | '{' | '}' | '<' | '>' | '/' | '%'
            );
        if valid {
            prefix.push(character);
        } else if !prefix.ends_with('-') {
            prefix.push('-');
        }
    }
    let prefix = prefix.trim_matches('-');
    let prefix = &prefix[..prefix.len().min(MAX_LEN - HASH_LEN - 1)];
    if prefix.is_empty() {
        hash
    } else {
        format!("{prefix}-{hash}")
    }
}

fn checked_gids(
    physical_count: usize,
    matrix: Vec<Vec<usize>>,
) -> Result<Vec<Box<[GlyphId16]>>, Error> {
    if physical_count >= usize::from(u16::MAX) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Variant font has too many physical glyphs.",
        ));
    }
    Ok(matrix
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|gid| GlyphId16::new(gid as u16))
                .collect()
        })
        .collect())
}

#[cfg(test)]
mod tests;
