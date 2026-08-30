use crate::{GenerateWebfontsOptions, ttf::generate_ttf_font_bytes, woff};

#[test]
fn generates_woff2_font_with_expected_header() {
    let ttf_result = generate_ttf_font_bytes(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![format!(
            "{}/../vite-svg-2-webfont/src/fixtures/webfont-test/svg/add.svg",
            env!("CARGO_MANIFEST_DIR")
        )],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        ..Default::default()
    })
    .expect("expected ttf generation to succeed");

    let result = woff::ttf_to_woff2(&ttf_result, 10).expect("woff2 generation should succeed");

    assert_eq!(&result[..4], b"wOF2");
}
