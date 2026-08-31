use crate::woff;

#[test]
fn generates_woff2_font_with_expected_header() {
    let tables = crate::test_helpers::fixture_font_tables();

    let result = woff::ttf_to_woff2(tables.ttf(), 10).expect("woff2 generation should succeed");

    assert_eq!(&result[..4], b"wOF2");
}
