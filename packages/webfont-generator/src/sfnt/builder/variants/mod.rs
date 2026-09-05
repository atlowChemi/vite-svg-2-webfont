use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};

use write_fonts::tables::cmap::Cmap;
use write_fonts::tables::fvar::{AxisInstanceArrays, Fvar, InstanceRecord, VariationAxisRecord};
use write_fonts::tables::glyf::SimpleGlyph;
use write_fonts::tables::gsub::{
    self, Gsub, SingleSubst, SubstitutionLookup, SubstitutionLookupList,
};
use write_fonts::tables::layout::builders::{Builder, LookupBuilder};
use write_fonts::tables::layout::{
    Condition, ConditionSet, CoverageTable, Feature, FeatureList, FeatureRecord,
    FeatureTableSubstitution, FeatureTableSubstitutionRecord, FeatureVariationRecord,
    FeatureVariations, LangSys, Lookup, LookupFlag, Script, ScriptList, ScriptRecord,
};
use write_fonts::tables::stat::{AxisRecord, AxisValue, AxisValueTableFlags, Stat};
use write_fonts::tables::variations::ivs_builder::VariationStoreBuilder;
use write_fonts::types::{F2Dot14, Fixed, GlyphId, GlyphId16, NameId, Tag};

use crate::input::ResolvedVariants;
use crate::sfnt::SerializedFontTables;
use crate::svg::types::{PreparedVariantFamily, ProcessedGlyph};

use super::clamp_to_u16;
use super::glyphs::{build_glyf_table, compile_simple_glyph, compute_glyph_metrics};
use super::ligatures;
use super::tables::{
    assemble_font, build_name_table, build_os2, derive_version_string, make_windows_name_record,
};
use super::types::{CompiledGlyph, CompiledGlyphOutline, TtfOptions};

const WEIGHT_NAME_ID: u16 = 256;
const FIRST_VARIANT_NAME_ID: u16 = 257;

pub(crate) struct VariantFontBuild {
    pub(crate) tables: SerializedFontTables,
    #[allow(
        dead_code,
        reason = "presentation GIDs are consumed by later variant phases"
    )]
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
    let ligature_placeholders =
        ligatures::build_ligature_placeholders(&physical[..family.glyphs.len()], options.ligature);
    let presentation_gids = checked_gids(physical.len() + ligature_placeholders.len(), matrix)?;

    let (glyf, loca, loca_format) = build_glyf_table(&physical, &ligature_placeholders)?;
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
        &ligature_placeholders,
        glyf,
        loca,
        loca_format,
        &metrics,
        family.ascent,
        family.descent,
        family.font_height,
        None,
    )?;

    let cmap =
        Cmap::from_mappings(
            family
                .glyphs
                .iter()
                .enumerate()
                .filter_map(|(logical_index, glyph)| {
                    char::from_u32(glyph.codepoint).map(|codepoint| {
                        (
                            codepoint,
                            GlyphId::new(u32::from(
                                presentation_gids[logical_index][variants.default_index].to_u16(),
                            )),
                        )
                    })
                })
                .chain(ligature_placeholders.iter().enumerate().filter_map(
                    |(index, placeholder)| {
                        char::from_u32(placeholder.codepoint).map(|codepoint| {
                            (codepoint, GlyphId::new((physical.len() + index + 1) as u32))
                        })
                    },
                )),
        )
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
    let gsub = build_variant_gsub(
        family,
        variants,
        &presentation_gids,
        &ligature_placeholders,
        physical.len(),
    );

    let mut tables = base
        .tables()
        .iter()
        .filter(|table| {
            table.tag != *b"GSUB"
                && table.tag != *b"OS/2"
                && table.tag != *b"cmap"
                && table.tag != *b"name"
        })
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
            family
                .glyphs
                .iter()
                .map(|glyph| glyph.codepoint)
                .chain(ligature_placeholders.iter().map(|glyph| glyph.codepoint)),
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
    tables.push((
        *b"GSUB",
        write_fonts::dump_table(&gsub).map_err(Error::other)?,
    ));

    Ok(VariantFontBuild {
        tables: SerializedFontTables::new(tables)?,
        presentation_gids,
    })
}

pub(crate) fn build_static_variant(
    options: TtfOptions<'_>,
    family: &PreparedVariantFamily,
    variant_index: usize,
    weight: u16,
) -> Result<SerializedFontTables, Error> {
    let glyphs = family
        .glyphs
        .iter()
        .enumerate()
        .map(|(index, glyph)| {
            let mut outline =
                glyph.outlines[variant_index]
                    .clone()
                    .unwrap_or_else(|| ProcessedGlyph {
                        codepoint: glyph.codepoint,
                        height: 0.0,
                        index,
                        name: glyph.name.clone(),
                        path_data: "".into(),
                        ttf_path: None,
                        ttf_path_hash: None,
                        width: glyph.advance_width,
                    });
            outline.codepoint = glyph.codepoint;
            outline.index = index;
            outline.name.clone_from(&glyph.name);
            outline.width = glyph.advance_width;
            outline
        })
        .collect::<Vec<_>>();
    let weight = weight.to_string();
    super::build(
        TtfOptions {
            ascent: Some(family.ascent),
            copyright: options.copyright,
            descent: Some(family.descent),
            description: options.description,
            font_height: Some(family.font_height),
            font_name: options.font_name,
            font_style: options.font_style,
            font_weight: Some(&weight),
            ligature: options.ligature,
            manufacturer_url: options.manufacturer_url,
            ts: options.ts,
            version: options.version,
        },
        &glyphs,
        None,
    )
}

fn build_variant_gsub(
    family: &PreparedVariantFamily,
    variants: &ResolvedVariants,
    presentation_gids: &[Box<[GlyphId16]>],
    placeholders: &[ligatures::LigaturePlaceholderGlyph],
    physical_count: usize,
) -> Gsub {
    let default_gids = presentation_gids
        .iter()
        .map(|row| row[variants.default_index])
        .collect::<Vec<_>>();
    let mut lookups = Vec::new();
    let mut rvrn_lookups = vec![None; variants.variants.len()];
    for variant_index in 0..variants.variants.len() {
        if variant_index == variants.default_index {
            continue;
        }
        rvrn_lookups[variant_index] = Some(lookups.len() as u16);
        lookups.push(SubstitutionLookup::Single(Lookup::new(
            LookupFlag::empty(),
            vec![SingleSubst::format_2(
                CoverageTable::format_1(default_gids.clone()),
                presentation_gids
                    .iter()
                    .map(|row| row[variant_index])
                    .collect(),
            )],
        )));
    }

    let mut liga_lookups = vec![None; variants.variants.len()];
    if !placeholders.is_empty() {
        for (variant_index, lookup_index) in liga_lookups.iter_mut().enumerate() {
            *lookup_index = Some(lookups.len() as u16);
            lookups.push(build_variant_ligature_lookup(
                family,
                presentation_gids,
                placeholders,
                physical_count,
                variant_index,
            ));
        }
    }

    let (features, rvrn_feature_index, liga_feature_index) =
        if let Some(default_liga) = liga_lookups[variants.default_index] {
            (
                vec![
                    FeatureRecord::new(Tag::new(b"liga"), Feature::new(None, vec![default_liga])),
                    FeatureRecord::new(Tag::new(b"rvrn"), Feature::new(None, vec![])),
                ],
                1,
                Some(0),
            )
        } else {
            (
                vec![FeatureRecord::new(
                    Tag::new(b"rvrn"),
                    Feature::new(None, vec![]),
                )],
                0,
                None,
            )
        };
    let mut gsub = Gsub::new(
        ScriptList::new(vec![ScriptRecord::new(
            Tag::new(b"DFLT"),
            Script::new(
                Some(LangSys::new((0..features.len() as u16).collect())),
                vec![],
            ),
        )]),
        FeatureList::new(features),
        SubstitutionLookupList::new(lookups),
    );
    gsub.feature_variations.set(FeatureVariations::new(
        variants
            .variants
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != variants.default_index)
            .map(|(variant_index, _)| {
                let mut substitutions = Vec::new();
                if let (Some(feature_index), Some(lookup_index)) =
                    (liga_feature_index, liga_lookups[variant_index])
                {
                    substitutions.push(FeatureTableSubstitutionRecord::new(
                        feature_index,
                        Feature::new(None, vec![lookup_index]),
                    ));
                }
                substitutions.push(FeatureTableSubstitutionRecord::new(
                    rvrn_feature_index,
                    Feature::new(None, vec![rvrn_lookups[variant_index].unwrap()]),
                ));
                let (minimum, maximum) = variant_range(variants, variant_index);
                FeatureVariationRecord::new(
                    Some(ConditionSet::new(vec![Condition::format_1_axis_range(
                        0, minimum, maximum,
                    )])),
                    Some(FeatureTableSubstitution::new(substitutions)),
                )
            })
            .collect(),
    ));
    gsub
}

fn build_variant_ligature_lookup(
    family: &PreparedVariantFamily,
    presentation_gids: &[Box<[GlyphId16]>],
    placeholders: &[ligatures::LigaturePlaceholderGlyph],
    physical_count: usize,
    variant_index: usize,
) -> SubstitutionLookup {
    let placeholder_gids = placeholders
        .iter()
        .enumerate()
        .map(|(index, glyph)| {
            (
                glyph.codepoint,
                GlyphId16::new((physical_count + index + 1) as u16),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut builder =
        LookupBuilder::<gsub::builders::LigatureSubBuilder>::new(LookupFlag::empty(), None);
    for (logical_index, glyph) in family.glyphs.iter().enumerate() {
        let sequence = glyph
            .name
            .chars()
            .filter_map(|character| placeholder_gids.get(&u32::from(character)).copied())
            .collect::<Vec<_>>();
        if sequence.len() >= 2 {
            builder
                .last_mut()
                .expect("ligature lookup builder should always contain a subtable")
                .insert(sequence, presentation_gids[logical_index][variant_index]);
        }
    }
    let mut variation_store = VariationStoreBuilder::new(0);
    SubstitutionLookup::Ligature(builder.build(&mut variation_store))
}

fn variant_range(variants: &ResolvedVariants, variant_index: usize) -> (F2Dot14, F2Dot14) {
    let boundary = |lower: usize, upper: usize| {
        let midpoint = (f64::from(variants.variants[lower].weight)
            + f64::from(variants.variants[upper].weight))
            / 2.0;
        let minimum = f64::from(variants.variants[0].weight);
        let default = f64::from(variants.variants[variants.default_index].weight);
        let maximum = f64::from(variants.variants.last().unwrap().weight);
        let normalized = if midpoint < default {
            (midpoint - default) / (default - minimum)
        } else {
            (midpoint - default) / (maximum - default)
        };
        F2Dot14::from_f64(normalized)
    };
    let minimum = if variant_index == 0 {
        F2Dot14::NEG_ONE
    } else {
        boundary(variant_index - 1, variant_index)
    };
    let maximum = if variant_index + 1 == variants.variants.len() {
        F2Dot14::ONE
    } else {
        let next_boundary = boundary(variant_index, variant_index + 1);
        F2Dot14::from_bits(next_boundary.to_bits() - 1)
    };
    (minimum, maximum)
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
