use std::collections::{BTreeMap, HashSet};

use write_fonts::tables::gsub::{
    Gsub, SubstitutionLookup, SubstitutionLookupList, builders::LigatureSubBuilder,
};
use write_fonts::tables::layout::builders::{Builder, LookupBuilder};
use write_fonts::tables::layout::{
    Feature, FeatureList, FeatureRecord, LangSys, LookupFlag, Script, ScriptList, ScriptRecord,
};
use write_fonts::tables::variations::ivs_builder::VariationStoreBuilder;
use write_fonts::types::{GlyphId16, Tag};

use super::types::CompiledGlyph;

pub(super) struct LigaturePlaceholderGlyph {
    pub(super) codepoint: u32,
    pub(super) name: String,
}

pub(super) fn build_ligature_placeholders(
    compiled_glyphs: &[CompiledGlyph],
    ligature: bool,
) -> Vec<LigaturePlaceholderGlyph> {
    if !ligature {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut placeholders = Vec::new();
    for glyph in compiled_glyphs {
        if glyph.name.chars().count() < 2 {
            continue;
        }
        for character in glyph.name.chars() {
            let codepoint = u32::from(character);
            if seen.insert(codepoint) {
                placeholders.push(LigaturePlaceholderGlyph {
                    codepoint,
                    name: format!("ligature-char-{:X}", codepoint),
                });
            }
        }
    }
    placeholders
}

pub(super) fn build_ligature_gsub(
    compiled_glyphs: &[CompiledGlyph],
    ligature_placeholders: &[LigaturePlaceholderGlyph],
) -> Option<Gsub> {
    if ligature_placeholders.is_empty() {
        return None;
    }
    let placeholder_glyph_ids = ligature_placeholders
        .iter()
        .enumerate()
        .map(|(index, glyph)| {
            (
                glyph.codepoint,
                GlyphId16::new((compiled_glyphs.len() + index + 1) as u16),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut lookup_builder = LookupBuilder::<LigatureSubBuilder>::new(LookupFlag::empty(), None);
    for (index, glyph) in compiled_glyphs.iter().enumerate() {
        let sequence = glyph
            .name
            .chars()
            .filter_map(|character| placeholder_glyph_ids.get(&u32::from(character)).copied())
            .collect::<Vec<_>>();
        if sequence.len() < 2 {
            continue;
        }
        lookup_builder
            .last_mut()
            .expect("ligature lookup builder should always contain a subtable")
            .insert(sequence, GlyphId16::new((index + 1) as u16));
    }
    if lookup_builder
        .iter_subtables()
        .all(LigatureSubBuilder::is_empty)
    {
        return None;
    }
    let script_list = ScriptList::new(vec![ScriptRecord::new(
        Tag::new(b"DFLT"),
        Script::new(Some(LangSys::new(vec![0])), vec![]),
    )]);
    let feature_list = FeatureList::new(vec![FeatureRecord::new(
        Tag::new(b"liga"),
        Feature::new(None, vec![0]),
    )]);
    let mut variation_store = VariationStoreBuilder::new(0);
    let lookup = lookup_builder.build(&mut variation_store);
    let lookup_list = SubstitutionLookupList::new(vec![SubstitutionLookup::Ligature(lookup)]);
    Some(Gsub::new(script_list, feature_list, lookup_list))
}
