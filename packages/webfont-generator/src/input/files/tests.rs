#[cfg(feature = "napi")]
use super::load_svg_files_napi;
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

#[cfg(feature = "napi")]
#[tokio::test]
async fn napi_loader_uses_default_glyph_names() {
    let path = std::env::temp_dir().join(format!("webfont-input-{}-icon.svg", std::process::id()));
    std::fs::write(&path, "<svg />").unwrap();
    let paths = vec![path.to_string_lossy().into_owned()];

    let source_files = load_svg_files_napi(&paths, None).await.unwrap();

    assert_eq!(
        source_files[0].glyph_name,
        format!("webfont-input-{}-icon", std::process::id())
    );
    assert_eq!(&*source_files[0].contents, "<svg />");
    std::fs::remove_file(path).unwrap();
}
