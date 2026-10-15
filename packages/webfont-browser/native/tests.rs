use super::*;
use serde_json::json;
use wasm_bindgen_test::wasm_bindgen_test;

const SVG: &str = r#"<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>"#;

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
