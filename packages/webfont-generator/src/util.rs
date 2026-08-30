use std::io::{Error, ErrorKind};
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

#[cfg(feature = "napi")]
use napi::{Error as NapiError, Status};

/// Convert any displayable error into a NAPI GenericFailure error.
#[cfg(feature = "napi")]
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
///
/// When `base_url` consists only of slashes (e.g. `"/"`), the leading slash
/// is preserved so absolute-from-root references like `cssFontsUrl: "/"`
/// produce URLs such as `"/iconfont.woff2"` rather than collapsing to a
/// relative path.
pub(crate) fn join_url(base_url: &str, file_name: &str) -> String {
    if base_url.is_empty() {
        return file_name.trim_start_matches('/').to_owned();
    }
    let trimmed_base = base_url.trim_end_matches('/');
    let trimmed_file = file_name.trim_start_matches('/');
    if trimmed_base.is_empty() {
        format!("/{trimmed_file}")
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

#[cfg(test)]
mod tests {
    use super::join_url;

    #[test]
    fn join_url_returns_file_name_when_base_is_empty() {
        assert_eq!(join_url("", "iconfont.woff2"), "iconfont.woff2");
        assert_eq!(join_url("", "/iconfont.woff2"), "iconfont.woff2");
    }

    #[test]
    fn join_url_preserves_leading_slash_when_base_is_root() {
        assert_eq!(join_url("/", "iconfont.woff2"), "/iconfont.woff2");
        assert_eq!(join_url("/", "/iconfont.woff2"), "/iconfont.woff2");
        assert_eq!(join_url("//", "iconfont.woff2"), "/iconfont.woff2");
    }

    #[test]
    fn join_url_joins_relative_and_absolute_paths() {
        assert_eq!(join_url("/foo", "iconfont.woff2"), "/foo/iconfont.woff2");
        assert_eq!(join_url("/foo/", "iconfont.woff2"), "/foo/iconfont.woff2");
        assert_eq!(join_url("foo", "iconfont.woff2"), "foo/iconfont.woff2");
        assert_eq!(join_url("foo/", "iconfont.woff2"), "foo/iconfont.woff2");
        assert_eq!(join_url("/foo", "/iconfont.woff2"), "/foo/iconfont.woff2");
    }

    #[test]
    fn join_url_preserves_absolute_url_origins() {
        assert_eq!(
            join_url("https://cdn.example.com", "iconfont.woff2"),
            "https://cdn.example.com/iconfont.woff2"
        );
        assert_eq!(
            join_url("https://cdn.example.com/", "iconfont.woff2"),
            "https://cdn.example.com/iconfont.woff2"
        );
        assert_eq!(
            join_url("https://cdn.example.com/assets", "iconfont.woff2"),
            "https://cdn.example.com/assets/iconfont.woff2"
        );
    }
}
