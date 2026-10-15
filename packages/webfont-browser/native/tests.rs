use super::*;
use serde_json::json;
use wasm_bindgen_test::wasm_bindgen_test;

const SVG: &str = r#"<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>"#;

#[wasm_bindgen_test]
fn rejects_invalid_browser_clock() {
    let date = js_sys::Reflect::get(&js_sys::global(), &"Date".into()).unwrap();
    let original = js_sys::Reflect::get(&date, &"now".into()).unwrap();
    let invalid = js_sys::Function::new_no_args("return NaN");
    js_sys::Reflect::set(&date, &"now".into(), &invalid).unwrap();
    let result = FontSession::new(
        &json!({"sources": [{"id": "icon", "name": "icon", "svg": SVG}], "types": ["ttf"]})
            .to_string(),
    );
    js_sys::Reflect::set(&date, &"now".into(), &original).unwrap();
    let error = result.err().expect("invalid clock must fail");
    assert!(
        js_sys::Error::from(error)
            .message()
            .includes("invalid timestamp", 0)
    );
}

#[wasm_bindgen_test]
fn generates_copied_outputs_and_recovers_from_errors() {
    let options = json!({
        "sources": [{"id": "opaque", "name": "square", "svg": SVG}],
        "types": ["svg", "ttf", "eot", "woff", "woff2"],
        "html": true, "incremental": true
    });
    let mut session = FontSession::new(&options.to_string()).unwrap();
    for format in ["svg", "ttf", "eot", "woff", "woff2"] {
        assert!(session.font(format).unwrap().unwrap().length() > 0);
    }
    assert!(session.font("unknown").is_err());
    assert!(session.css().unwrap().contains("@font-face"));
    assert!(session.html().unwrap().contains("square"));
    assert_eq!(Object::keys(&session.outputs().unwrap()).length(), 7);
    let original = session.font("ttf").unwrap().unwrap();
    let saved = original.to_vec();
    assert!(session.regenerate("invalid", "[]").is_err());
    assert!(
        session
            .regenerate(r#"{"sources":["opaque"]}"#, "invalid")
            .is_err()
    );
    assert!(
        session
            .regenerate(r#"{"sources":["missing"]}"#, "[]")
            .is_err()
    );
    session
        .regenerate(r#"{"sources":["opaque"]}"#, "[]")
        .unwrap();
    assert_eq!(original.to_vec(), saved);
    original.fill(0, 0, original.length());
    assert_eq!(session.font("ttf").unwrap().unwrap().to_vec(), saved);
}

#[wasm_bindgen_test]
fn validates_options_and_keeps_templates_lazy() {
    assert!(FontSession::new("invalid").is_err());
    assert!(FontSession::new(r#"{"files":[]}"#).is_err());
    assert!(FontSession::new(r#"{"sources":[]}"#).is_err());
    let session = FontSession::new(
        &json!({
            "sources": [{"id": "opaque", "name": "square", "svg": SVG}],
            "types": ["ttf"], "formatOptions": {"ttf": {"ts": 0}},
            "cssTemplate": "{{#if", "htmlTemplate": "{{#if", "html": true
        })
        .to_string(),
    )
    .unwrap();
    assert!(session.font("woff").unwrap().is_none());
    assert!(session.css().is_err());
    assert!(session.html().is_err());
    assert!(session.outputs().is_err());
    assert!(session.font("ttf").unwrap().is_some());
}

#[wasm_bindgen_test]
fn single_actions_use_retained_sources_and_preserve_state_on_error() {
    let options = json!({
        "sources": [{"id": "first", "name": "first", "svg": SVG}],
        "types": ["ttf"], "incremental": true, "formatOptions": {"ttf": {"ts": 0}}
    });
    let mut session = FontSession::new(&options.to_string()).unwrap();
    assert!(session.add("invalid", None).is_err());
    let source = json!({"id": "second", "name": "second", "svg": SVG}).to_string();
    session.add(&source, None).unwrap();
    assert!(session.add(&source, None).is_err());
    session
        .update("second".into(), SVG.into(), Some("renamed".into()))
        .unwrap();
    assert!(session.css().unwrap().contains("renamed"));
    session.remove("first".into()).unwrap();
    let saved = session.font("ttf").unwrap().unwrap().to_vec();
    assert!(session.remove("second".into()).is_err());
    assert!(
        session
            .update("second".into(), "<broken".into(), None)
            .is_err()
    );
    assert_eq!(session.font("ttf").unwrap().unwrap().to_vec(), saved);
}
