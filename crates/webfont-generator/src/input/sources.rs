use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::sync::Arc;

use super::options::resolve_codepoints;
use crate::types::MissingGlyphBehavior;

/// Content consumed by the shared pipeline. IDs are opaque cache keys, not filesystem paths.
/// Names belong to memberships: file hooks may give one ID different names across variants.
#[derive(Clone)]
pub(crate) struct LoadedSvgSource {
    pub contents: Arc<str>,
    pub glyph_name: String,
    pub id: String,
}

pub(crate) struct VariantFamilySources<Files = Vec<LoadedSvgSource>> {
    pub variants: Vec<Files>,
    pub glyphs: Vec<LogicalGlyphSources>,
}

pub(crate) struct LogicalGlyphSources {
    pub name: String,
    pub codepoint: u32,
    pub sources: Box<[Option<VariantGlyphSource>]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VariantGlyphSource {
    Source {
        variant_index: usize,
        source_index: usize,
    },
    Blank,
}

pub(crate) fn build_variant_family_sources<Files: AsRef<[LoadedSvgSource]>>(
    variants: Vec<Files>,
    explicit_codepoints: &BTreeMap<String, u32>,
    start_codepoint: u32,
) -> std::io::Result<(VariantFamilySources<Files>, BTreeMap<String, u32>)> {
    let mut name_to_index = HashMap::new();
    let mut glyphs = Vec::<(String, Box<[Option<VariantGlyphSource>]>)>::new();

    for (variant_index, files) in variants.iter().enumerate() {
        for (source_index, file) in files.as_ref().iter().enumerate() {
            let glyph_index = *name_to_index
                .entry(file.glyph_name.clone())
                .or_insert_with(|| {
                    let index = glyphs.len();
                    glyphs.push((
                        file.glyph_name.clone(),
                        vec![None; variants.len()].into_boxed_slice(),
                    ));
                    index
                });
            glyphs[glyph_index].1[variant_index] = Some(VariantGlyphSource::Source {
                variant_index,
                source_index,
            });
        }
    }

    let codepoints = resolve_codepoints(
        glyphs.iter().map(|(name, _)| name.as_str()),
        explicit_codepoints,
        start_codepoint,
    )?;
    let glyphs = glyphs
        .into_iter()
        .map(|(name, sources)| LogicalGlyphSources {
            codepoint: codepoints[&name],
            name,
            sources,
        })
        .collect();

    Ok((VariantFamilySources { variants, glyphs }, codepoints))
}

pub(crate) fn resolve_missing_glyphs<Files>(
    family: &mut VariantFamilySources<Files>,
    behavior: MissingGlyphBehavior,
    fallback_index: Option<usize>,
    variant_names: &[&str],
) -> std::io::Result<()> {
    if behavior == MissingGlyphBehavior::Error {
        let missing = family
            .glyphs
            .iter()
            .flat_map(|glyph| {
                glyph
                    .sources
                    .iter()
                    .enumerate()
                    .filter(|(_, source)| source.is_none())
                    .map(|(variant_index, _)| {
                        format!(
                            "\"{}\" in variant \"{}\"",
                            glyph.name, variant_names[variant_index]
                        )
                    })
            })
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Missing glyphs: {}.", missing.join(", ")),
            ));
        }
        return Ok(());
    }

    if behavior == MissingGlyphBehavior::Fallback {
        let fallback_index = fallback_index.expect("validated fallback must resolve");
        if let Some(glyph) = family
            .glyphs
            .iter()
            .find(|glyph| glyph.sources[fallback_index].is_none())
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Fallback variant \"{}\" is missing glyph \"{}\".",
                    variant_names[fallback_index], glyph.name
                ),
            ));
        }
    }

    for glyph in &mut family.glyphs {
        let replacement = if behavior == MissingGlyphBehavior::Blank {
            VariantGlyphSource::Blank
        } else {
            let fallback_index = fallback_index.expect("validated fallback must resolve");
            glyph.sources[fallback_index].expect("fallback completeness checked above")
        };
        for source in &mut glyph.sources {
            source.get_or_insert(replacement);
        }
    }

    Ok(())
}

pub(crate) fn validate_glyph_names(source_files: &[LoadedSvgSource]) -> std::io::Result<()> {
    let mut seen_names = HashSet::with_capacity(source_files.len());

    for source_file in source_files {
        if !seen_names.insert(source_file.glyph_name.clone()) {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "The glyph name \"{}\" must be unique.",
                    source_file.glyph_name
                ),
            ));
        }
    }

    Ok(())
}
#[cfg(test)]
mod tests;
