#[cfg(test)]
mod tests;

use std::io::{Error, ErrorKind};
use std::path::Path;

use crate::GenerationHooks;
use tokio::task::JoinSet;

use super::sources::{LoadedSvgSource, validate_glyph_names};

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

#[cfg(test)]
pub(crate) async fn load_variant_svg_files(
    variant_paths: &[Vec<String>],
    rename: Option<&(dyn Fn(&str) -> String + Send + Sync)>,
) -> std::io::Result<Vec<Vec<LoadedSvgSource>>> {
    load_variant_svg_files_with_hooks(variant_paths, &crate::hooks::RenameHooks(rename)).await
}

/// Resolve glyph names via a runtime-independent asynchronous hook.
pub(crate) async fn load_svg_files_with_hooks<H: GenerationHooks>(
    paths: &[String],
    hooks: &H,
    validate_names: bool,
) -> Result<Vec<LoadedSvgSource>, H::Error> {
    let raw = load_svg_contents(paths).await?;
    let glyph_names = if let Some(glyph_names) = hooks.rename(paths).await? {
        if glyph_names.len() != raw.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "rename callback returned an unexpected number of glyph names".to_owned(),
            )
            .into());
        }
        glyph_names
    } else {
        raw.iter()
            .map(|(path, _)| default_glyph_name_from_path(path))
            .collect::<std::io::Result<_>>()?
    };
    let source_files = raw
        .into_iter()
        .zip(glyph_names)
        .map(|((path, contents), glyph_name)| LoadedSvgSource {
            contents: contents.into(),
            glyph_name,
            // File mode uses the verbatim path as its cache identity. Actual file provenance
            // remains in resolved file/variant membership, outside the shared source record.
            id: path,
        })
        .collect::<Vec<_>>();

    if validate_names {
        validate_glyph_names(&source_files)?;
    }
    Ok(source_files)
}

pub(crate) async fn load_variant_svg_files_with_hooks<H: GenerationHooks>(
    variant_paths: &[Vec<String>],
    hooks: &H,
) -> Result<Vec<Vec<LoadedSvgSource>>, H::Error> {
    let lengths = variant_paths.iter().map(Vec::len).collect::<Vec<_>>();
    let paths = variant_paths.iter().flatten().cloned().collect::<Vec<_>>();
    let source_files = load_svg_files_with_hooks(&paths, hooks, false).await?;

    Ok(split_loaded_variant_files(source_files, &lengths)?)
}

fn split_loaded_variant_files(
    source_files: Vec<LoadedSvgSource>,
    lengths: &[usize],
) -> std::io::Result<Vec<Vec<LoadedSvgSource>>> {
    let mut source_files = source_files.into_iter();
    let mut variants = Vec::with_capacity(lengths.len());
    for length in lengths {
        let variant = source_files.by_ref().take(*length).collect::<Vec<_>>();
        validate_glyph_names(&variant)?;
        variants.push(variant);
    }
    Ok(variants)
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
#[cfg(test)]
fn glyph_name_from_path(
    path: &str,
    rename: Option<&(dyn Fn(&str) -> String + Send + Sync)>,
) -> Result<String, Error> {
    match rename {
        Some(rename) => Ok(rename(path)),
        None => default_glyph_name_from_path(path),
    }
}
