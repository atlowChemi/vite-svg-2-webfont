use std::collections::HashMap;

use crate::types::GlyphChangeEntry;
use webfont_generator::{FontType, GlyphChange};

use super::{parse_glyph_changes, parse_native_urls, to_napi_err};

fn options() -> webfont_generator::GenerateWebfontsOptions {
    webfont_generator::GenerateWebfontsOptions {
        dest: "artifacts".into(),
        files: vec![format!(
            "{}/../vite-svg-2-webfont/src/fixtures/webfont-test/svg/add.svg",
            env!("CARGO_MANIFEST_DIR")
        )],
        types: Some(vec![
            FontType::Svg,
            FontType::Ttf,
            FontType::Eot,
            FontType::Woff,
            FontType::Woff2,
        ]),
        incremental: Some(true),
        write_files: Some(false),
        ..Default::default()
    }
}

fn incremental_result() -> super::GenerateWebfontsResult {
    super::GenerateWebfontsResult(webfont_generator::generate_sync(options(), None).unwrap())
}

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
napi_stub!(napi_delete_reference(_env: napi::sys::napi_env, _reference: napi::sys::napi_ref));
napi_stub!(napi_reference_unref(_env: napi::sys::napi_env, _reference: napi::sys::napi_ref, _result: *mut u32));
napi_stub!(napi_call_threadsafe_function(_function: napi::sys::napi_threadsafe_function, _data: *mut std::ffi::c_void, _mode: napi::sys::napi_threadsafe_function_call_mode));
napi_stub!(napi_release_threadsafe_function(_function: napi::sys::napi_threadsafe_function, _mode: napi::sys::napi_threadsafe_function_release_mode));

fn change(path: String, change_type: &str, name: Option<&str>) -> GlyphChangeEntry {
    GlyphChangeEntry {
        path,
        change_type: change_type.to_owned(),
        name: name.map(str::to_owned),
    }
}

#[test]
fn napi_font_getters_return_generated_and_absent_formats() {
    let result = incremental_result();

    assert_eq!(result.svg().as_deref(), result.0.svg_string());
    assert_eq!(result.ttf().as_deref(), result.0.ttf_bytes());
    assert_eq!(result.eot().as_deref(), result.0.eot_bytes());
    assert_eq!(result.woff().as_deref(), result.0.woff_bytes());
    assert_eq!(result.woff2().as_deref(), result.0.woff2_bytes());

    let mut empty_options = options();
    empty_options.types = Some(vec![]);
    let result = super::GenerateWebfontsResult(
        webfont_generator::generate_sync(empty_options, None).unwrap(),
    );
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
    let files = options().files;

    result
        .regenerate_from_js(
            crate::types::RegenerationFileOptions {
                files: Some(files.clone()),
                variants: None,
            },
            Some(vec![change(files[0].clone(), "changed", None)]),
        )
        .unwrap();
    result
        .regenerate_from_js(
            crate::types::RegenerationFileOptions {
                files: Some(files),
                variants: None,
            },
            None,
        )
        .unwrap();
    assert!(result.svg().unwrap().contains("glyph-name=\"add\""));
}

#[tokio::test]
async fn asynchronous_regeneration_succeeds_and_restores_state_after_failure() {
    let result =
        super::GenerateWebfontsResult(webfont_generator::generate(options(), None).await.unwrap());
    let files = options().files;
    let replacement = result
        .regenerate_async_from_js(
            crate::types::RegenerationFileOptions {
                files: Some(files.clone()),
                variants: None,
            },
            None,
        )
        .await
        .unwrap();
    assert!(replacement.svg().is_some());

    let missing = format!(
        "{}/missing-result-test-{}.svg",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let error = replacement
        .regenerate_async_from_js(
            crate::types::RegenerationFileOptions {
                files: Some(files.clone()),
                variants: None,
            },
            Some(vec![change(missing, "changed", None)]),
        )
        .await
        .err()
        .unwrap();
    assert!(!error.reason.is_empty());

    let retried = replacement
        .regenerate_async_from_js(
            crate::types::RegenerationFileOptions {
                files: Some(files),
                variants: None,
            },
            None,
        )
        .await
        .unwrap();
    assert!(retried.svg().is_some());
}
