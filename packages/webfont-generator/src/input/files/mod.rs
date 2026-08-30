#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "napi")]
use napi::threadsafe_function::ThreadsafeFunction;
#[cfg(feature = "napi")]
use napi::{Error as NapiError, Status};
use tokio::task::JoinSet;

#[derive(Clone)]
pub(crate) struct LoadedSvgFile {
    pub contents: Arc<str>,
    pub glyph_name: String,
    pub path: String,
}

#[cfg(feature = "napi")]
fn to_napi_err(error: impl std::fmt::Display) -> NapiError {
    NapiError::new(Status::GenericFailure, error.to_string())
}

/// Load SVG file contents in parallel, preserving the original order.
async fn load_svg_contents(paths: &[String]) -> std::io::Result<Vec<(String, String)>> {
    let mut tasks = JoinSet::new();

    for (index, path) in paths.iter().cloned().enumerate() {
        tasks.spawn(async move {
            tokio::fs::read_to_string(&path)
                .await
                .map(|contents| (index, (path, contents)))
        });
    }

    let mut results = Vec::with_capacity(paths.len());
    while let Some(result) = tasks.join_next().await {
        let (index, pair) = result
            .map_err(|error| std::io::Error::other(format!("SVG loading task failed: {error}")))?
            .map_err(|error| {
                std::io::Error::other(format!("Failed to read source SVG file: {error}"))
            })?;
        results.push((index, pair));
    }

    results.sort_by_key(|(index, _)| *index);
    Ok(results.into_iter().map(|(_, pair)| pair).collect())
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

/// NAPI version: resolve glyph names via async ThreadsafeFunction callback.
#[cfg(feature = "napi")]
#[allow(clippy::type_complexity)]
pub(crate) async fn load_svg_files_napi(
    paths: &[String],
    rename: Option<&ThreadsafeFunction<Vec<String>, Vec<String>, Vec<String>, Status, false>>,
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

    validate_glyph_names(&source_files).map_err(to_napi_err)?;
    Ok(source_files)
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
