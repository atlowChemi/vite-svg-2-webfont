#[cfg(feature = "napi")]
use super::load_svg_files_napi;
use super::{
    LoadedSvgFile, build_variant_family_sources, default_glyph_name_from_path,
    glyph_name_from_path, load_svg_contents, load_variant_svg_files,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

fn loaded(path: &str, glyph_name: &str) -> LoadedSvgFile {
    LoadedSvgFile {
        contents: Arc::from("<svg />"),
        glyph_name: glyph_name.to_owned(),
        path: path.to_owned(),
    }
}

fn temp_svg(name: &str) -> String {
    let path =
        std::env::temp_dir().join(format!("webfont-variant-{}-{name}.svg", std::process::id()));
    std::fs::write(&path, "<svg />").unwrap();
    path.to_string_lossy().into_owned()
}

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

#[tokio::test]
async fn reports_file_loading_errors_in_source_order() {
    let paths = vec![
        "/missing/first.svg".to_owned(),
        "/missing/second.svg".to_owned(),
    ];

    let error = load_svg_contents(&paths).await.unwrap_err();

    assert!(error.to_string().contains("/missing/first.svg"));
}

#[cfg(feature = "napi")]
#[tokio::test]
async fn napi_loader_uses_default_glyph_names() {
    let path = std::env::temp_dir().join(format!("webfont-input-{}-icon.svg", std::process::id()));
    std::fs::write(&path, "<svg />").unwrap();
    let paths = vec![path.to_string_lossy().into_owned()];

    let source_files = load_svg_files_napi(&paths, None, true).await.unwrap();

    assert_eq!(
        source_files[0].glyph_name,
        format!("webfont-input-{}-icon", std::process::id())
    );
    assert_eq!(&*source_files[0].contents, "<svg />");
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn variant_loader_preserves_flattened_rename_order() {
    let a = temp_svg("a");
    let b = temp_svg("b");
    let c = temp_svg("c");
    let paths = vec![vec![b.clone(), a.clone()], vec![c.clone(), b.clone()]];
    let calls = Mutex::new(Vec::new());
    let rename = |path: &str| {
        let mut calls = calls.lock().unwrap();
        calls.push(path.to_owned());
        format!("glyph-{}", calls.len())
    };

    let variants = load_variant_svg_files(&paths, Some(&rename)).await.unwrap();

    assert_eq!(
        *calls.lock().unwrap(),
        [b.clone(), a.clone(), c.clone(), b.clone()]
    );
    assert_eq!(
        variants
            .iter()
            .map(|files| files
                .iter()
                .map(|file| file.glyph_name.as_str())
                .collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec!["glyph-1", "glyph-2"], vec!["glyph-3", "glyph-4"]],
    );
    for path in [a, b, c] {
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn variant_loader_validates_names_within_each_variant() {
    let path = temp_svg("shared");

    let error = load_variant_svg_files(
        &[vec![path.clone(), path.clone()], vec![path.clone()]],
        None,
    )
    .await
    .err()
    .expect("expected duplicate names within one variant to fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("must be unique"));

    let variants = load_variant_svg_files(&[vec![path.clone()], vec![path.clone()]], None)
        .await
        .unwrap();
    assert_eq!(variants[0][0].glyph_name, variants[1][0].glyph_name);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn builds_sparse_logical_glyphs_in_first_appearance_order() {
    let variants = vec![
        vec![loaded("small/b.svg", "b"), loaded("small/a.svg", "a")],
        vec![loaded("large/a.svg", "a"), loaded("large/c.svg", "c")],
    ];
    let explicit = BTreeMap::from([("a".to_owned(), 42)]);

    let (family, codepoints) = build_variant_family_sources(variants, &explicit, 41).unwrap();

    assert_eq!(
        family
            .glyphs
            .iter()
            .map(|glyph| glyph.name.as_str())
            .collect::<Vec<_>>(),
        ["b", "a", "c"],
    );
    assert_eq!(
        family
            .glyphs
            .iter()
            .map(|glyph| glyph.codepoint)
            .collect::<Vec<_>>(),
        [41, 42, 43],
    );
    assert_eq!(&*family.glyphs[0].sources, [Some(0), None]);
    assert_eq!(&*family.glyphs[1].sources, [Some(1), Some(0)]);
    assert_eq!(&*family.glyphs[2].sources, [None, Some(1)]);
    assert_eq!(family.variants[1][0].path, "large/a.svg");
    assert_eq!(
        codepoints,
        BTreeMap::from([
            ("a".to_owned(), 42),
            ("b".to_owned(), 41),
            ("c".to_owned(), 43)
        ])
    );
}

#[test]
fn rebuilds_union_codepoints_from_the_explicit_base() {
    let explicit = BTreeMap::new();
    let (_, initial) = build_variant_family_sources(
        vec![vec![loaded("small/a.svg", "a"), loaded("small/b.svg", "b")]],
        &explicit,
        100,
    )
    .unwrap();
    let (_, changed) = build_variant_family_sources(
        vec![vec![loaded("small/b.svg", "b"), loaded("small/c.svg", "c")]],
        &explicit,
        100,
    )
    .unwrap();

    assert_eq!(
        initial,
        BTreeMap::from([("a".to_owned(), 100), ("b".to_owned(), 101)])
    );
    assert_eq!(
        changed,
        BTreeMap::from([("b".to_owned(), 100), ("c".to_owned(), 101)])
    );
}
