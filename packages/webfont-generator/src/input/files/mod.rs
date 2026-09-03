#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "napi")]
use napi::threadsafe_function::ThreadsafeFunction;
#[cfg(feature = "napi")]
use napi::{Error as NapiError, Status};
use tokio::task::JoinSet;

use super::options::resolve_codepoints;
use crate::types::MissingGlyphBehavior;

#[derive(Clone)]
pub(crate) struct LoadedSvgFile {
    pub contents: Arc<str>,
    pub glyph_name: String,
    pub path: String,
}

#[allow(
    dead_code,
    reason = "variant sources are consumed by later generation phases"
)]
pub(crate) struct VariantFamilySources {
    pub variants: Vec<Vec<LoadedSvgFile>>,
    pub glyphs: Vec<LogicalGlyphSources>,
}

#[allow(
    dead_code,
    reason = "logical glyphs are consumed by later generation phases"
)]
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

#[cfg(feature = "napi")]
fn to_napi_err(error: impl std::fmt::Display) -> NapiError {
    NapiError::new(Status::GenericFailure, error.to_string())
}

/// Load SVG file contents in parallel, preserving the original order.
async fn load_svg_contents(paths: &[String]) -> std::io::Result<Vec<(String, String)>> {
    let mut tasks = JoinSet::new();

    for (index, path) in paths.iter().cloned().enumerate() {
        tasks.spawn(async move { (index, path.clone(), tokio::fs::read_to_string(path).await) });
    }

    let mut results = Vec::with_capacity(paths.len());
    while let Some(result) = tasks.join_next().await {
        results.push(
            result.map_err(|error| {
                std::io::Error::other(format!("SVG loading task failed: {error}"))
            })?,
        );
    }

    results.sort_by_key(|(index, _, _)| *index);
    results
        .into_iter()
        .map(|(_, path, contents)| match contents {
            Ok(contents) => Ok((path, contents)),
            Err(error) => Err(std::io::Error::other(format!(
                "Failed to read source SVG file '{path}': {error}"
            ))),
        })
        .collect()
}

/// Load SVG files and resolve glyph names using an optional sync rename function.
pub(crate) async fn load_svg_files(
    paths: &[String],
    rename: Option<&(dyn Fn(&str) -> String + Send + Sync)>,
) -> std::io::Result<Vec<LoadedSvgFile>> {
    let raw = load_svg_contents(paths).await?;
    let source_files: Vec<LoadedSvgFile> = raw
        .into_iter()
        .map(|(path, contents)| {
            let glyph_name = glyph_name_from_path(&path, rename)?;
            Ok(LoadedSvgFile {
                contents: contents.into(),
                glyph_name,
                path,
            })
        })
        .collect::<std::io::Result<_>>()?;

    validate_glyph_names(&source_files)?;
    Ok(source_files)
}

pub(crate) async fn load_variant_svg_files(
    variant_paths: &[Vec<String>],
    rename: Option<&(dyn Fn(&str) -> String + Send + Sync)>,
) -> std::io::Result<Vec<Vec<LoadedSvgFile>>> {
    let lengths = variant_paths.iter().map(Vec::len).collect::<Vec<_>>();
    let paths = variant_paths.iter().flatten().cloned().collect::<Vec<_>>();
    let raw = load_svg_contents(&paths).await?;
    let glyph_names = raw
        .iter()
        .map(|(path, _)| glyph_name_from_path(path, rename))
        .collect::<std::io::Result<Vec<_>>>()?;

    split_variant_files(raw, glyph_names, &lengths)
}

/// NAPI version: resolve glyph names via async ThreadsafeFunction callback.
#[cfg(feature = "napi")]
#[allow(clippy::type_complexity)]
pub(crate) async fn load_svg_files_napi(
    paths: &[String],
    rename: Option<&ThreadsafeFunction<Vec<String>, Vec<String>, Vec<String>, Status, false>>,
    validate_names: bool,
) -> napi::Result<Vec<LoadedSvgFile>> {
    let raw = load_svg_contents(paths).await.map_err(to_napi_err)?;
    let glyph_names = if let Some(rename) = rename {
        let glyph_names = rename.call_async_catch(paths.to_vec()).await?;
        if glyph_names.len() != raw.len() {
            return Err(NapiError::new(
                Status::InvalidArg,
                "rename callback returned an unexpected number of glyph names".to_owned(),
            ));
        }
        glyph_names
    } else {
        raw.iter()
            .map(|(path, _)| default_glyph_name_from_path(path).map_err(to_napi_err))
            .collect::<napi::Result<_>>()?
    };
    let source_files = raw
        .into_iter()
        .zip(glyph_names)
        .map(|((path, contents), glyph_name)| LoadedSvgFile {
            contents: contents.into(),
            glyph_name,
            path,
        })
        .collect::<Vec<_>>();

    if validate_names {
        validate_glyph_names(&source_files).map_err(to_napi_err)?;
    }
    Ok(source_files)
}

#[cfg(feature = "napi")]
#[allow(clippy::type_complexity)]
pub(crate) async fn load_variant_svg_files_napi(
    variant_paths: &[Vec<String>],
    rename: Option<&ThreadsafeFunction<Vec<String>, Vec<String>, Vec<String>, Status, false>>,
) -> napi::Result<Vec<Vec<LoadedSvgFile>>> {
    let lengths = variant_paths.iter().map(Vec::len).collect::<Vec<_>>();
    let paths = variant_paths.iter().flatten().cloned().collect::<Vec<_>>();
    let source_files = load_svg_files_napi(&paths, rename, false).await?;

    split_loaded_variant_files(source_files, &lengths).map_err(to_napi_err)
}

fn split_variant_files(
    raw: Vec<(String, String)>,
    glyph_names: Vec<String>,
    lengths: &[usize],
) -> std::io::Result<Vec<Vec<LoadedSvgFile>>> {
    let source_files = raw
        .into_iter()
        .zip(glyph_names)
        .map(|((path, contents), glyph_name)| LoadedSvgFile {
            contents: contents.into(),
            glyph_name,
            path,
        });
    split_loaded_variant_files(source_files.collect(), lengths)
}

fn split_loaded_variant_files(
    source_files: Vec<LoadedSvgFile>,
    lengths: &[usize],
) -> std::io::Result<Vec<Vec<LoadedSvgFile>>> {
    let mut source_files = source_files.into_iter();
    let mut variants = Vec::with_capacity(lengths.len());
    for length in lengths {
        let variant = source_files.by_ref().take(*length).collect::<Vec<_>>();
        validate_glyph_names(&variant)?;
        variants.push(variant);
    }
    Ok(variants)
}

pub(crate) fn build_variant_family_sources(
    variants: Vec<Vec<LoadedSvgFile>>,
    explicit_codepoints: &BTreeMap<String, u32>,
    start_codepoint: u32,
) -> std::io::Result<(VariantFamilySources, BTreeMap<String, u32>)> {
    let mut name_to_index = HashMap::new();
    let mut glyphs = Vec::<(String, Box<[Option<VariantGlyphSource>]>)>::new();

    for (variant_index, files) in variants.iter().enumerate() {
        for (source_index, file) in files.iter().enumerate() {
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

pub(crate) fn resolve_missing_glyphs(
    family: &mut VariantFamilySources,
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
        let replacement = match behavior {
            MissingGlyphBehavior::Blank => VariantGlyphSource::Blank,
            MissingGlyphBehavior::Fallback => {
                let fallback_index = fallback_index.expect("validated fallback must resolve");
                glyph.sources[fallback_index].expect("fallback completeness checked above")
            }
            MissingGlyphBehavior::Error => unreachable!(),
        };
        for source in &mut glyph.sources {
            source.get_or_insert(replacement);
        }
    }

    Ok(())
}

pub(crate) fn validate_glyph_names(source_files: &[LoadedSvgFile]) -> std::io::Result<()> {
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

fn default_glyph_name_from_path(path: &str) -> Result<String, Error> {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Unable to derive glyph name from '{path}'."),
            )
        })
}

/// Resolve a glyph name from a file path, optionally applying a rename function.
fn glyph_name_from_path(
    path: &str,
    rename: Option<&(dyn Fn(&str) -> String + Send + Sync)>,
) -> Result<String, Error> {
    match rename {
        Some(rename) => Ok(rename(path)),
        None => default_glyph_name_from_path(path),
    }
}
