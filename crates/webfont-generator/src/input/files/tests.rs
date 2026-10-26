use super::load_svg_files_with_hooks;
use super::{
    default_glyph_name_from_path, glyph_name_from_path, load_svg_contents, load_variant_svg_files,
};
use crate::input::{
    LoadedSvgSource, VariantGlyphSource, build_variant_family_sources, resolve_missing_glyphs,
};
use crate::types::MissingGlyphBehavior;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

fn loaded(path: &str, glyph_name: &str) -> LoadedSvgSource {
    LoadedSvgSource {
        contents: Arc::from("<svg />"),
        glyph_name: glyph_name.to_owned(),
        id: path.to_owned(),
    }
}

fn source(variant_index: usize, source_index: usize) -> Option<VariantGlyphSource> {
    Some(VariantGlyphSource::Source {
        variant_index,
        source_index,
    })
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

    let error = load_svg_contents(&paths, None).await.unwrap_err();

    assert!(error.to_string().contains("/missing/first.svg"));
}

/// Removes a test's temporary folder even when an assertion fails.
struct TempFolder(std::path::PathBuf);

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn loads_many_files_across_tasks_in_source_order() {
    let folder =
        TempFolder(std::env::temp_dir().join(format!("webfont-load-order-{}", std::process::id())));
    std::fs::create_dir_all(&folder.0).unwrap();
    // 100 files over 4 tasks of 25. The first task's files are large, so it very likely finishes
    // last and the result is only in source order if the chunks are put back in order.
    let files: Vec<(String, String)> = (0..100)
        .map(|index| {
            let path = folder.0.join(format!("{index}.svg"));
            let padding = if index < 25 { 200_000 } else { 0 };
            let contents = format!("<svg id=\"{index}\">{}</svg>", " ".repeat(padding));
            std::fs::write(&path, &contents).unwrap();
            (path.to_string_lossy().into_owned(), contents)
        })
        .collect();
    let paths: Vec<String> = files.iter().map(|(path, _)| path.clone()).collect();
    let four = std::num::NonZeroUsize::new(4);

    assert_eq!(load_svg_contents(&paths, four).await.unwrap(), files);
    // Unset uses one task per core (at most one per 16 files): same result.
    assert_eq!(load_svg_contents(&paths, None).await.unwrap(), files);

    // Missing files in different chunks: the first one in source order is reported.
    let mut broken = paths.clone();
    broken[40] = "/missing/forty.svg".to_owned();
    broken[90] = "/missing/ninety.svg".to_owned();
    let error = load_svg_contents(&broken, four).await.unwrap_err();
    assert!(error.to_string().contains("/missing/forty.svg"));
}

#[tokio::test]
async fn hooked_loader_uses_default_glyph_names() {
    let path = std::env::temp_dir().join(format!("webfont-input-{}-icon.svg", std::process::id()));
    std::fs::write(&path, "<svg />").unwrap();
    let paths = vec![path.to_string_lossy().into_owned()];

    let source_files = load_svg_files_with_hooks(&paths, &(), true, None)
        .await
        .unwrap();

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
    assert_eq!(&*family.glyphs[0].sources, [source(0, 0), None]);
    assert_eq!(&*family.glyphs[1].sources, [source(0, 1), source(1, 0)]);
    assert_eq!(&*family.glyphs[2].sources, [None, source(1, 1)]);
    assert_eq!(family.variants[1][0].id, "large/a.svg");
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

#[test]
fn resolves_missing_glyphs_as_explicit_blanks() {
    let (mut family, _) = build_variant_family_sources(
        vec![
            vec![loaded("small/b.svg", "b"), loaded("small/a.svg", "a")],
            vec![loaded("large/a.svg", "a"), loaded("large/c.svg", "c")],
        ],
        &BTreeMap::from([("a".to_owned(), 42)]),
        41,
    )
    .unwrap();

    resolve_missing_glyphs(
        &mut family,
        MissingGlyphBehavior::Blank,
        None,
        &["small", "large"],
    )
    .unwrap();

    assert_eq!(
        family
            .glyphs
            .iter()
            .map(|glyph| (glyph.name.as_str(), glyph.codepoint))
            .collect::<Vec<_>>(),
        [("b", 41), ("a", 42), ("c", 43)]
    );
    assert_eq!(
        &*family.glyphs[0].sources,
        [source(0, 0), Some(VariantGlyphSource::Blank)]
    );
    assert_eq!(&*family.glyphs[1].sources, [source(0, 1), source(1, 0)]);
    assert_eq!(
        &*family.glyphs[2].sources,
        [Some(VariantGlyphSource::Blank), source(1, 1)]
    );
}

#[test]
fn reports_all_missing_glyphs_in_glyph_then_variant_order() {
    let (mut family, _) = build_variant_family_sources(
        vec![
            vec![loaded("small/a.svg", "a")],
            vec![loaded("medium/b.svg", "b")],
            vec![loaded("large/c.svg", "c")],
        ],
        &BTreeMap::new(),
        100,
    )
    .unwrap();

    let error = resolve_missing_glyphs(
        &mut family,
        MissingGlyphBehavior::Error,
        None,
        &["small", "medium", "large"],
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Missing glyphs: \"a\" in variant \"medium\", \"a\" in variant \"large\", \"b\" in variant \"small\", \"b\" in variant \"large\", \"c\" in variant \"small\", \"c\" in variant \"medium\"."
    );
}

#[test]
fn resolves_only_missing_cells_from_the_fallback_variant() {
    let (mut family, _) = build_variant_family_sources(
        vec![
            vec![loaded("small/a.svg", "a")],
            vec![loaded("large/a.svg", "a"), loaded("large/b.svg", "b")],
        ],
        &BTreeMap::new(),
        100,
    )
    .unwrap();

    resolve_missing_glyphs(
        &mut family,
        MissingGlyphBehavior::Fallback,
        Some(1),
        &["small", "large"],
    )
    .unwrap();

    assert_eq!(&*family.glyphs[0].sources, [source(0, 0), source(1, 0)]);
    assert_eq!(&*family.glyphs[1].sources, [source(1, 1), source(1, 1)]);
}

#[test]
fn rejects_a_glyph_missing_from_the_fallback_variant() {
    let (mut family, _) = build_variant_family_sources(
        vec![
            vec![loaded("small/a.svg", "a")],
            vec![loaded("large/b.svg", "b")],
        ],
        &BTreeMap::new(),
        100,
    )
    .unwrap();

    let error = resolve_missing_glyphs(
        &mut family,
        MissingGlyphBehavior::Fallback,
        Some(1),
        &["small", "large"],
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Fallback variant \"large\" is missing glyph \"a\"."
    );
}

#[test]
fn leaves_complete_families_unchanged_for_every_policy() {
    for behavior in [
        MissingGlyphBehavior::Blank,
        MissingGlyphBehavior::Error,
        MissingGlyphBehavior::Fallback,
    ] {
        let (mut family, _) = build_variant_family_sources(
            vec![
                vec![loaded("small/a.svg", "a")],
                vec![loaded("large/a.svg", "a")],
            ],
            &BTreeMap::new(),
            100,
        )
        .unwrap();
        let before = family.glyphs[0].sources.clone();

        resolve_missing_glyphs(&mut family, behavior, Some(0), &["small", "large"]).unwrap();

        assert_eq!(family.glyphs[0].sources, before);
    }
}
