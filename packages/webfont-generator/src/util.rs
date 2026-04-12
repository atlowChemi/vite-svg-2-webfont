use std::collections::{BTreeMap, HashSet};
use std::io::{Error, ErrorKind};
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use napi::threadsafe_function::ThreadsafeFunction;
use napi::{Error as NapiError, Status};

use crate::types::LoadedSvgFile;

/// Convert any displayable error into a NAPI GenericFailure error.
#[inline]
pub(crate) fn to_napi_err(error: impl std::fmt::Display) -> NapiError {
    NapiError::new(Status::GenericFailure, error.to_string())
}

/// Convert any displayable error into an `io::Error` with `InvalidData` kind.
#[inline]
pub(crate) fn to_io_err(error: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::InvalidData, error.to_string())
}

/// Temporarily swap a field in a Handlebars Context, run a render closure,
/// then restore the original value. Avoids cloning the entire Context.
#[inline]
pub(crate) fn render_with_field_swap<F>(
    ctx: &mut handlebars::Context,
    key: &str,
    value: Value,
    render: F,
) -> Result<String, Error>
where
    F: FnOnce(&handlebars::Context) -> Result<String, Error>,
{
    let obj = ctx
        .data_mut()
        .as_object_mut()
        .expect("context should be an object");
    let original = obj.insert(key.to_owned(), value);
    let result = render(ctx);
    let obj = ctx.data_mut().as_object_mut().unwrap();
    match original {
        Some(v) => {
            obj.insert(key.to_owned(), v);
        }
        None => {
            obj.remove(key);
        }
    }
    result
}

/// Join a base URL with a file name, normalizing slashes.
pub(crate) fn join_url(base_url: &str, file_name: &str) -> String {
    let trimmed_base = base_url.trim_end_matches('/');
    let trimmed_file = file_name.trim_start_matches('/');
    if trimmed_base.is_empty() {
        trimmed_file.to_owned()
    } else {
        format!("{trimmed_base}/{trimmed_file}")
    }
}

/// Compute a relative path from `from` to `to`.
pub(crate) fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_components = from.components().collect::<Vec<_>>();
    let to_components = to.components().collect::<Vec<_>>();
    let common_prefix_len = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(left, right)| left == right)
        .count();

    let mut result = PathBuf::new();
    for _ in &from_components[common_prefix_len..] {
        result.push("..");
    }
    for component in &to_components[common_prefix_len..] {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => result.push("."),
            Component::ParentDir => result.push(".."),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    result
}

/// Convert a path to use forward slashes (for URLs on Windows).
#[inline]
pub(crate) fn path_to_slashes(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
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

pub(crate) async fn glyph_name_from_path(
    path: &str,
    rename: Option<&ThreadsafeFunction<String, String, String, Status, false>>,
) -> napi::Result<String> {
    if let Some(rename) = rename {
        rename.call_async(path.to_owned()).await
    } else {
        default_glyph_name_from_path(path)
            .map_err(|error| NapiError::new(Status::InvalidArg, error.to_string()))
    }
}

pub(crate) fn resolve_codepoints(
    source_files: &[LoadedSvgFile],
    codepoints: &BTreeMap<String, u32>,
    start_codepoint: u32,
) -> Result<BTreeMap<String, u32>, Error> {
    let mut resolved_codepoints = codepoints.clone();
    let mut used_codepoints: HashSet<u32> = resolved_codepoints.values().copied().collect();
    let mut next_codepoint = start_codepoint;

    for source_file in source_files {
        let name = source_file.glyph_name.clone();

        if resolved_codepoints.contains_key(&name) {
            continue;
        }

        while used_codepoints.contains(&next_codepoint) {
            next_codepoint += 1;
        }

        resolved_codepoints.insert(name, next_codepoint);
        used_codepoints.insert(next_codepoint);
        next_codepoint += 1;
    }

    Ok(resolved_codepoints)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::{glyph_name_from_path, resolve_codepoints};
    use crate::types::LoadedSvgFile;
    use napi::Status;

    fn loaded_svg_file(path: &str) -> LoadedSvgFile {
        LoadedSvgFile {
            contents: "<svg />".to_owned(),
            glyph_name: Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned(),
            path: path.to_owned(),
        }
    }

    #[test]
    fn derives_glyph_name_from_path() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let glyph_name = runtime
            .block_on(glyph_name_from_path("/tmp/icons/arrow-left.svg", None))
            .unwrap();

        assert_eq!(glyph_name, "arrow-left");
    }

    #[test]
    fn errors_when_glyph_name_cannot_be_derived() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let error = runtime
            .block_on(glyph_name_from_path("/tmp/icons/..", None))
            .unwrap_err();

        assert_eq!(error.status, Status::InvalidArg);
        assert!(error
            .to_string()
            .contains("Unable to derive glyph name from '/tmp/icons/..'."));
    }

    #[test]
    fn resolves_missing_codepoints_in_source_file_order() {
        let source_files = vec![
            loaded_svg_file("/tmp/icons/arrow-left.svg"),
            loaded_svg_file("/tmp/icons/arrow-right.svg"),
        ];

        let resolved_codepoints =
            resolve_codepoints(&source_files, &BTreeMap::new(), 0xF101).unwrap();

        assert_eq!(resolved_codepoints.get("arrow-left"), Some(&0xF101));
        assert_eq!(resolved_codepoints.get("arrow-right"), Some(&0xF102));
    }

    #[test]
    fn preserves_explicit_codepoints_and_skips_used_values() {
        let source_files = vec![
            loaded_svg_file("/tmp/icons/arrow-left.svg"),
            loaded_svg_file("/tmp/icons/arrow-right.svg"),
            loaded_svg_file("/tmp/icons/check.svg"),
        ];
        let explicit_codepoints = BTreeMap::from([
            ("arrow-left".to_owned(), 0xF105),
            ("check".to_owned(), 0xF101),
        ]);

        let resolved_codepoints =
            resolve_codepoints(&source_files, &explicit_codepoints, 0xF101).unwrap();

        assert_eq!(resolved_codepoints.get("arrow-left"), Some(&0xF105));
        assert_eq!(resolved_codepoints.get("check"), Some(&0xF101));
        assert_eq!(resolved_codepoints.get("arrow-right"), Some(&0xF102));
    }

    #[test]
    fn errors_when_any_source_file_has_no_usable_file_stem() {
        let source_files = vec![LoadedSvgFile {
            contents: "<svg />".to_owned(),
            glyph_name: String::new(),
            path: "/tmp/icons/..".to_owned(),
        }];

        let resolved_codepoints =
            resolve_codepoints(&source_files, &BTreeMap::new(), 0xF101).unwrap();
        assert_eq!(resolved_codepoints.get(""), Some(&0xF101));
    }
}
