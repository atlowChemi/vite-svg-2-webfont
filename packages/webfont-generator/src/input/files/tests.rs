use super::{default_glyph_name_from_path, glyph_name_from_path};

#[test]
fn derives_glyph_name_from_path() {
    let glyph_name = glyph_name_from_path("/tmp/icons/arrow-left.svg", None).unwrap();

    assert_eq!(glyph_name, "arrow-left");
}

#[test]
fn errors_when_glyph_name_cannot_be_derived() {
    let error = default_glyph_name_from_path("/tmp/icons/..").unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("Unable to derive glyph name from '/tmp/icons/..'.")
    );
}
