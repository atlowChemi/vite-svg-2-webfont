use std::io::{Error, ErrorKind};

use serde_json::Value;

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
