use std::collections::HashMap;

use crate::result::tests::incremental_result;
use crate::types::{FontType, GlyphChange, GlyphChangeEntry};

use super::{parse_glyph_changes, parse_native_urls, to_napi_err};

// Direct adapter tests link without Node, so satisfy NAPI error symbols that remain reachable.
macro_rules! napi_stub {
    ($name:ident($($argument:ident: $type:ty),*)) => {
        #[unsafe(no_mangle)]
        extern "C" fn $name($($argument: $type),*) -> napi::sys::napi_status {
            0
        }
    };
}

napi_stub!(napi_create_error(
    _env: napi::sys::napi_env,
    _code: napi::sys::napi_value,
    _message: napi::sys::napi_value,
    _result: *mut napi::sys::napi_value
));
napi_stub!(napi_create_string_utf8(
    _env: napi::sys::napi_env,
    _string: *const std::ffi::c_char,
    _length: isize,
    _result: *mut napi::sys::napi_value
));
napi_stub!(napi_get_and_clear_last_exception(
    _env: napi::sys::napi_env,
    _result: *mut napi::sys::napi_value
));
napi_stub!(napi_get_named_property(
    _env: napi::sys::napi_env,
    _object: napi::sys::napi_value,
    _name: *const std::ffi::c_char,
    _result: *mut napi::sys::napi_value
));
napi_stub!(napi_get_reference_value(
    _env: napi::sys::napi_env,
    _reference: napi::sys::napi_ref,
    _result: *mut napi::sys::napi_value
));
napi_stub!(napi_is_error(
    _env: napi::sys::napi_env,
    _value: napi::sys::napi_value,
    _result: *mut bool
));
napi_stub!(napi_is_exception_pending(
    _env: napi::sys::napi_env,
    _result: *mut bool
));
napi_stub!(napi_set_named_property(
    _env: napi::sys::napi_env,
    _object: napi::sys::napi_value,
    _name: *const std::ffi::c_char,
    _value: napi::sys::napi_value
));
napi_stub!(napi_throw(
    _env: napi::sys::napi_env,
    _error: napi::sys::napi_value
));

fn change(path: String, change_type: &str, name: Option<&str>) -> GlyphChangeEntry {
    GlyphChangeEntry {
        path,
        change_type: change_type.to_owned(),
        name: name.map(str::to_owned),
    }
}

#[test]
fn napi_font_getters_return_generated_and_absent_formats() {
    let mut result = incremental_result();

    assert_eq!(result.svg().as_deref(), result.svg_string());
    assert_eq!(result.ttf().as_deref(), result.ttf_bytes());
    assert_eq!(result.eot().as_deref(), result.eot_bytes());
    assert_eq!(result.woff().as_deref(), result.woff_bytes());
    assert_eq!(result.woff2().as_deref(), result.woff2_bytes());

    result.fonts.svg_font = None;
    result.fonts.ttf_font = None;
    result.fonts.eot_font = None;
    result.fonts.woff_font = None;
    result.fonts.woff2_font = None;
    assert!(result.svg().is_none());
    assert!(result.ttf().is_none());
    assert!(result.eot().is_none());
    assert!(result.woff().is_none());
    assert!(result.woff2().is_none());
}

#[test]
fn napi_render_methods_parse_known_urls_and_ignore_unknown_ones() {
    let result = incremental_result();
    let urls = HashMap::from([
        ("svg".to_owned(), "/cdn/result.svg".to_owned()),
        ("unknown".to_owned(), "/ignored".to_owned()),
    ]);

    assert!(result.generate_css(None).unwrap().contains("@font-face"));
    assert!(
        result
            .generate_css(Some(urls.clone()))
            .unwrap()
            .contains("/cdn/result.svg")
    );
    assert!(
        result
            .generate_html(Some(urls))
            .unwrap()
            .contains("/cdn/result.svg")
    );

    let parsed = parse_native_urls(HashMap::from([
        ("svg".to_owned(), "svg-url".to_owned()),
        ("ttf".to_owned(), "ttf-url".to_owned()),
        ("eot".to_owned(), "eot-url".to_owned()),
        ("woff".to_owned(), "woff-url".to_owned()),
        ("woff2".to_owned(), "woff2-url".to_owned()),
        ("other".to_owned(), "ignored".to_owned()),
    ]))
    .unwrap();
    assert_eq!(parsed.len(), 5);
    assert_eq!(parsed[&FontType::Woff2], "woff2-url");
}

#[test]
fn glyph_changes_parse_all_variants_and_reject_unknown_types() {
    assert!(parse_glyph_changes(None).unwrap().is_none());
    let parsed = parse_glyph_changes(Some(vec![
        change("add.svg".to_owned(), "added", Some("plus")),
        change("change.svg".to_owned(), "changed", None),
        change("remove.svg".to_owned(), "removed", Some("ignored")),
    ]))
    .unwrap()
    .unwrap();

    assert!(matches!(
        &parsed[0],
        (path, GlyphChange::Added { name: Some(name) }) if path == "add.svg" && name == "plus"
    ));
    assert!(matches!(
        &parsed[1],
        (path, GlyphChange::Changed { name: None }) if path == "change.svg"
    ));
    assert!(matches!(
        &parsed[2],
        (path, GlyphChange::Removed) if path == "remove.svg"
    ));

    let error = parse_glyph_changes(Some(vec![change("bad.svg".to_owned(), "renamed", None)]))
        .err()
        .unwrap();
    assert!(error.reason.contains("Unknown changeType 'renamed'"));
}

#[test]
fn napi_error_preserves_the_message() {
    let error = to_napi_err(std::io::Error::other("native failure"));
    assert_eq!(error.reason, "native failure");
}

#[test]
fn synchronous_regeneration_accepts_explicit_and_omitted_changes() {
    let mut result = incremental_result();
    let files = result.options.files.clone();

    result
        .regenerate_from_js(
            files.clone(),
            Some(vec![change(files[0].clone(), "changed", None)]),
        )
        .unwrap();
    result.regenerate_from_js(files, None).unwrap();
    assert!(result.svg().unwrap().contains("glyph-name=\"add\""));
}

#[tokio::test]
async fn asynchronous_regeneration_succeeds_and_restores_state_after_failure() {
    let result = incremental_result();
    let files = result.options.files.clone();
    let replacement = result
        .regenerate_async_from_js(files.clone(), None)
        .await
        .unwrap();
    assert!(replacement.svg().is_some());

    let missing = format!(
        "{}/missing-result-test-{}.svg",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let error = replacement
        .regenerate_async_from_js(files.clone(), Some(vec![change(missing, "changed", None)]))
        .await
        .err()
        .unwrap();
    assert!(!error.reason.is_empty());

    let retried = replacement
        .regenerate_async_from_js(files, None)
        .await
        .unwrap();
    assert!(retried.svg().is_some());
}
