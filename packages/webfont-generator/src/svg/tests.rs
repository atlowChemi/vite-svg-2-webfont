use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kurbo::Shape;
use write_fonts::read::tables::glyf::Glyph;
use write_fonts::read::{FontRef, TableProvider};
use write_fonts::types::{GlyphId, Tag};

use super::types::{GlyphCache, PreparedVariantFamily, ProcessedGlyph};
use super::{
    build_svg_font, glyph_scale, prepare_svg_font, prepare_svg_font_incremental,
    prepare_variant_svg_family, source_content_hash, svg_options_from_options,
};

use crate::input::{self, LoadedSvgFile, ResolvedGenerateWebfontsOptions, VariantFamilySources};
use crate::{
    FontType, FormatOptions, GenerateWebfontsOptions, MissingGlyphBehavior, SvgFormatOptions,
};

#[derive(Clone, Copy)]
struct SvgParityCase {
    fixture_dir: &'static str,
    expected_svg: &'static str,
    ascent: Option<f64>,
    center_horizontally: bool,
    center_vertically: bool,
    fixed_width: bool,
    font_id: Option<&'static str>,
    font_style: Option<&'static str>,
    font_weight: Option<&'static str>,
    normalize: Option<bool>,
    preserve_aspect_ratio: bool,
}

#[cfg(feature = "napi")]
use napi::sys::{napi_env, napi_ref, napi_status};

#[cfg(feature = "napi")]
#[unsafe(no_mangle)]
extern "C" fn napi_delete_reference(_: napi_env, _: napi_ref) -> napi_status {
    0
}

#[cfg(feature = "napi")]
#[unsafe(no_mangle)]
extern "C" fn napi_reference_unref(_: napi_env, _: napi_ref, result: *mut u32) -> napi_status {
    if !result.is_null() {
        // SAFETY: test-only stub writes a deterministic zero refcount when the pointer is valid.
        unsafe {
            *result = 0;
        }
    }
    0
}

impl SvgParityCase {
    const fn simple(fixture_dir: &'static str, expected_svg: &'static str) -> Self {
        Self {
            fixture_dir,
            expected_svg,
            ascent: None,
            center_horizontally: false,
            center_vertically: false,
            fixed_width: false,
            font_id: None,
            font_style: None,
            font_weight: None,
            normalize: None,
            preserve_aspect_ratio: false,
        }
    }
}

fn generate_svg_font(options: GenerateWebfontsOptions) -> String {
    let mut resolved_options = input::resolve_generate_webfonts_options(options)
        .unwrap_or_else(|error| panic!("native options should resolve: {error}"));
    resolved_options.types = vec![crate::FontType::Svg];
    let source_files = load_source_files(&resolved_options.files);
    input::finalize_generate_webfonts_options(&mut resolved_options, &source_files)
        .unwrap_or_else(|error| panic!("native codepoints should resolve: {error}"));
    let svg_options = svg_options_from_options(&resolved_options);
    let prepared = prepare_svg_font(&svg_options, &source_files)
        .unwrap_or_else(|error| panic!("native generation should succeed: {error}"));
    build_svg_font(&svg_options, &prepared)
}

fn load_source_files(paths: &[String]) -> Vec<LoadedSvgFile> {
    paths
        .iter()
        .map(|path| LoadedSvgFile {
            contents: fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("failed to load source SVG '{}': {error}", path))
                .into(),
            glyph_name: Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned(),
            path: path.clone(),
        })
        .collect()
}

fn run_case(case: SvgParityCase) {
    let fixture_dir = icons_root().join(case.fixture_dir);
    let result = generate_svg_font(GenerateWebfontsOptions {
        ascent: case.ascent,
        center_horizontally: Some(case.center_horizontally),
        center_vertically: Some(case.center_vertically),
        css: Some(false),
        dest: "artifacts".to_string(),
        files: svg_files(&fixture_dir),
        fixed_width: Some(case.fixed_width),
        format_options: Some(FormatOptions {
            svg: Some(SvgFormatOptions {
                font_id: case.font_id.map(str::to_string),
                ..Default::default()
            }),
            ..Default::default()
        }),
        html: Some(false),
        font_name: Some(case.fixture_dir.to_string()),
        font_style: case.font_style.map(str::to_string),
        font_weight: case.font_weight.map(str::to_string),
        ligature: Some(false),
        normalize: case.normalize,
        preserve_aspect_ratio: Some(case.preserve_aspect_ratio),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    assert_snapshot(case.expected_svg, &result);
}

macro_rules! svg_parity_test {
    ($name:ident, $fixture:literal, $expected:literal $(, $field:ident = $value:expr )* $(,)?) => {
        #[test]
        fn $name() {
            run_case(SvgParityCase {
                fixture_dir: $fixture,
                expected_svg: $expected,
                $($field: $value,)*
                ..SvgParityCase::simple($fixture, $expected)
            });
        }
    };
}

svg_parity_test!(
    originalicons_matches_upstream_contract,
    "originalicons",
    "originalicons.svg"
);
svg_parity_test!(
    cleanicons_matches_upstream_contract,
    "cleanicons",
    "cleanicons.svg"
);
svg_parity_test!(
    cleanicons_custom_ascent_matches_upstream_contract,
    "cleanicons",
    "cleanicons-ascent.svg",
    ascent = Some(100.0)
);
svg_parity_test!(
    cleanicons_font_style_and_weight_match_upstream_contract,
    "cleanicons",
    "cleanicons-stw.svg",
    font_style = Some("italic"),
    font_weight = Some("bold")
);
svg_parity_test!(
    multipathicons_matches_upstream_contract,
    "multipathicons",
    "multipathicons.svg"
);
svg_parity_test!(
    shapeicons_matches_upstream_contract,
    "shapeicons",
    "shapeicons.svg"
);
svg_parity_test!(
    variableheighticons_matches_upstream_contract,
    "variableheighticons",
    "variableheighticons.svg"
);
svg_parity_test!(
    variableheighticons_normalized_matches_upstream_contract,
    "variableheighticons",
    "variableheighticonsn.svg",
    normalize = Some(true)
);
svg_parity_test!(
    variableheighticons_preserve_aspect_ratio_matches_snapshot,
    "variableheighticons",
    "variableheighticonsnp.svg",
    normalize = Some(true),
    preserve_aspect_ratio = true
);
svg_parity_test!(
    preserveaspectratio_default_stretches_to_viewport_snapshot,
    "preserveaspectratio",
    "preserveaspectratio.svg"
);
svg_parity_test!(
    preserveaspectratio_true_preserves_viewbox_aspect_snapshot,
    "preserveaspectratio",
    "preserveaspectratio-preserved.svg",
    preserve_aspect_ratio = true
);
svg_parity_test!(
    variablewidthicons_matches_upstream_contract,
    "variablewidthicons",
    "variablewidthicons.svg"
);
svg_parity_test!(
    variablewidthicons_fixed_width_centered_matches_upstream_contract,
    "variablewidthicons",
    "variablewidthiconsn.svg",
    center_horizontally = true,
    fixed_width = true
);
svg_parity_test!(
    variablewidthicons_custom_font_id_matches_upstream_contract,
    "variablewidthicons",
    "variablewidthiconsid.svg",
    center_horizontally = true,
    fixed_width = true,
    font_id = Some("plop")
);
svg_parity_test!(
    tocentericons_matches_upstream_contract,
    "tocentericons",
    "tocentericons.svg",
    center_horizontally = true
);
svg_parity_test!(
    toverticalcentericons_matches_upstream_contract,
    "toverticalcentericons",
    "toverticalcentericons.svg",
    center_vertically = true
);
svg_parity_test!(
    hiddenpathesicons_matches_upstream_contract,
    "hiddenpathesicons",
    "hiddenpathesicons.svg"
);
svg_parity_test!(
    transformedicons_matches_upstream_contract,
    "transformedicons",
    "transformedicons.svg"
);
svg_parity_test!(
    pathfillnone_matches_upstream_contract,
    "pathfillnone",
    "pathfillnone.svg"
);
svg_parity_test!(
    roundedcorners_matches_upstream_contract,
    "roundedcorners",
    "roundedcorners.svg"
);
svg_parity_test!(
    rotatedrectangle_matches_upstream_contract,
    "rotatedrectangle",
    "rotatedrectangle.svg"
);
svg_parity_test!(paesku_matches_upstream_contract, "paesku", "paesku.svg");
svg_parity_test!(
    translatex_matches_upstream_contract,
    "translatex",
    "translatex.svg"
);
svg_parity_test!(skew_matches_upstream_contract, "skew", "skew.svg");
svg_parity_test!(
    onlywithrx_matches_upstream_contract,
    "onlywithrx",
    "onlywithrx.svg"
);
svg_parity_test!(
    onlywithry_matches_upstream_contract,
    "onlywithry",
    "onlywithry.svg"
);

use crate::test_helpers::{icons_root, webfont_fixture};

fn expected_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/svg/fixtures/expected")
        .canonicalize()
        .expect("native expected snapshot root should exist")
}

fn assert_snapshot(name: &str, actual_svg: &str) {
    let path = expected_root().join(name);
    if crate::test_helpers::update_snapshots_enabled() {
        fs::write(&path, actual_svg).unwrap_or_else(|error| {
            panic!("failed to update snapshot '{}': {error}", path.display())
        });
    }

    let expected_svg = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read expected snapshot '{}': {error}",
            path.display()
        )
    });
    assert_eq!(
        actual_svg, expected_svg,
        "svg snapshot mismatch for '{name}'"
    );
}

#[test]
fn zero_dimensions_keep_the_default_glyph_scale() {
    assert_eq!(glyph_scale(0.0, 0.0, true, 0.0, 1000.0), 1.0);
    assert_eq!(glyph_scale(0.0, 0.0, false, 0.0, 1000.0), 1.0);
}

#[test]
fn explicit_codepoints_are_used_without_ligatures() {
    let options = GenerateWebfontsOptions {
        codepoints: Some(HashMap::from([("add".to_string(), 0xF201)])),
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![webfont_fixture("add.svg")],
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_string()),
        html: Some(false),
        ligature: Some(false),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        ..Default::default()
    };
    let result = generate_svg_font(options.clone());
    let mut resolved_options = input::resolve_generate_webfonts_options(options.clone())
        .unwrap_or_else(|error| panic!("native options should resolve: {error}"));
    let source_files = load_source_files(&resolved_options.files);
    input::finalize_generate_webfonts_options(&mut resolved_options, &source_files)
        .unwrap_or_else(|error| panic!("native codepoints should resolve: {error}"));
    let svg_options = svg_options_from_options(&resolved_options);
    let prepared = prepare_svg_font(&svg_options, &source_files)
        .unwrap_or_else(|error| panic!("native preparation should succeed: {error}"));

    assert_snapshot("native-explicit-codepoints.svg", &result);
    assert_eq!(prepared.processed_glyphs[0].codepoint, 0xF201);
}

#[test]
fn ligatures_add_secondary_glyph_entries() {
    let result = generate_svg_font(GenerateWebfontsOptions {
        codepoints: Some(HashMap::from([("add".to_string(), 0xF201)])),
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![webfont_fixture("add.svg")],
        html: Some(false),
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_string()),
        ligature: Some(true),
        normalize: Some(true),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    assert_snapshot("native-ligatures.svg", &result);
}

#[test]
fn custom_font_height_and_descent_change_font_face_metrics() {
    let result = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        descent: Some(150.0),
        dest: "artifacts".to_string(),
        files: vec![webfont_fixture("add.svg"), webfont_fixture("test.svg")],
        html: Some(false),
        font_height: Some(1200.0),
        font_name: Some("iconfont".to_string()),
        ligature: Some(false),
        normalize: Some(true),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    assert_snapshot("native-font-height-descent.svg", &result);
}

#[test]
fn metadata_is_emitted_into_the_svg_font() {
    let result = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![webfont_fixture("add.svg")],
        format_options: Some(FormatOptions {
            svg: Some(SvgFormatOptions {
                metadata: Some("native-metadata".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        html: Some(false),
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_string()),
        ligature: Some(false),
        normalize: Some(true),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    assert_snapshot("native-metadata.svg", &result);
}

#[test]
fn optimize_output_updates_both_glyphs_and_svg_font_path_data() {
    let base = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![webfont_fixture("add.svg")],
        html: Some(false),
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_string()),
        ligature: Some(false),
        normalize: Some(true),
        optimize_output: Some(false),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });
    let optimized = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![webfont_fixture("add.svg")],
        html: Some(false),
        font_height: Some(1000.0),
        font_name: Some("iconfont".to_string()),
        ligature: Some(false),
        normalize: Some(true),
        optimize_output: Some(true),
        round: Some(1e3),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    assert_snapshot("native-optimize-output.svg", &optimized);
    let base_paths = attribute_values(&base, "<glyph ", "d");
    let optimized_paths = attribute_values(&optimized, "<glyph ", "d");

    assert!(!optimized_paths[0].is_empty());
    assert!(optimized_paths[0].len() <= base_paths[0].len());
    assert!(optimized.len() <= base.len());
}

fn svg_files(dir: &Path) -> Vec<String> {
    let mut files = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read fixture dir '{}': {error}", dir.display()))
        .map(|entry| entry.expect("dir entry should be readable").path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("svg"))
        .collect::<Vec<_>>();
    files.sort();
    files
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

fn attribute_values(svg: &str, tag_start: &str, attribute: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remainder = svg;

    while let Some(index) = remainder.find(tag_start) {
        let tag_remainder = &remainder[index..];
        let Some(tag_end) = tag_remainder.find('>') else {
            break;
        };
        let tag = &tag_remainder[..=tag_end];
        if let Some(value) = attribute_value(tag, tag_start, attribute) {
            values.push(value);
        }
        remainder = &tag_remainder[tag_end + 1..];
    }

    values
}

fn attribute_value(svg: &str, tag_start: &str, attribute: &str) -> Option<String> {
    let start = svg.find(tag_start)?;
    let tag_remainder = &svg[start..];
    let tag_end = tag_remainder.find('>')?;
    let tag = &tag_remainder[..=tag_end];
    let needle = format!("{attribute}=\"");
    let value_start = tag.find(&needle)? + needle.len();
    let value_end = tag[value_start..].find('"')?;
    Some(tag[value_start..value_start + value_end].to_string())
}

#[test]
fn empty_svg_produces_glyph_with_empty_path_data() {
    let fixture = icons_root()
        .join("emptyicons/empty.svg")
        .to_string_lossy()
        .into_owned();
    let svg = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![fixture],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    assert!(svg.contains("<glyph glyph-name=\"empty\""));
    let paths = attribute_values(&svg, "<glyph ", "d");
    assert_eq!(paths.len(), 1);
    assert!(
        paths[0].is_empty(),
        "empty SVG should produce empty path data"
    );
}

#[test]
fn empty_svg_mixed_with_normal_svgs_produces_valid_font() {
    let svg = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![
            icons_root()
                .join("cleanicons/plus.svg")
                .to_string_lossy()
                .into_owned(),
            icons_root()
                .join("emptyicons/empty.svg")
                .to_string_lossy()
                .into_owned(),
        ],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    let glyph_names = attribute_values(&svg, "<glyph ", "glyph-name");
    assert_eq!(glyph_names, vec!["plus", "empty"]);
    let paths = attribute_values(&svg, "<glyph ", "d");
    assert_eq!(paths.len(), 2);
    assert!(!paths[0].is_empty(), "plus glyph should have path data");
    assert!(
        paths[1].is_empty(),
        "empty glyph should have empty path data"
    );
}

#[test]
fn svg_font_does_not_deduplicate_identical_glyphs() {
    let icon = icons_root()
        .join("cleanicons/plus.svg")
        .to_string_lossy()
        .into_owned();
    let tmp = std::env::temp_dir().join(format!("svg-no-dedup-test-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();
    let copy_path = tmp.join("plus-copy.svg");
    fs::copy(&icon, &copy_path).unwrap();

    let svg = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        codepoints: Some(HashMap::from([
            ("plus".to_owned(), 0xE001u32),
            ("plus-copy".to_owned(), 0xE002u32),
        ])),
        dest: "artifacts".to_owned(),
        files: vec![icon, copy_path.to_string_lossy().into_owned()],
        html: Some(false),
        font_name: Some("iconfont".to_owned()),
        ligature: Some(false),
        start_codepoint: Some(0xE001),
        ..Default::default()
    });

    let _ = fs::remove_dir_all(&tmp);

    // SVG font should keep both glyphs as separate entries (dedup only applies to TTF)
    let glyph_names = attribute_values(&svg, "<glyph ", "glyph-name");
    assert_eq!(
        glyph_names,
        vec!["plus", "plus-copy"],
        "SVG font should not deduplicate — each glyph gets its own <glyph> element"
    );
}

#[test]
fn incremental_prepare_matches_full_prepare_and_reuses_cache() {
    let dir = icons_root().join("cleanicons");
    let make_options = || GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: svg_files(&dir),
        html: Some(false),
        font_name: Some("iconfont".to_string()),
        ligature: Some(false),
        ..Default::default()
    };

    let mut resolved = input::resolve_generate_webfonts_options(make_options()).unwrap();
    let mut source_files = load_source_files(&resolved.files);
    input::finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    let renamed_codepoint = resolved.codepoints[&source_files[0].glyph_name];
    resolved
        .codepoints
        .insert("renamed".to_string(), renamed_codepoint);
    let svg_options = svg_options_from_options(&resolved);

    let mut cache = GlyphCache::default();
    let error = prepare_svg_font_incremental(&svg_options, &[], &mut cache)
        .err()
        .unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Expected at least one SVG file for native generation."
    );

    let mut missing_codepoint = source_files[0].clone();
    missing_codepoint.glyph_name = "missing".to_string();
    let error = prepare_svg_font_incremental(&svg_options, &[missing_codepoint], &mut cache)
        .err()
        .unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        error.to_string(),
        "Missing resolved codepoint for glyph 'missing'."
    );
    assert_eq!(
        cache.parse_count, 0,
        "validation must happen before parsing"
    );

    // The SVG font string encodes every glyph's path data plus the font metrics, so equality
    // proves the prepared fonts match.
    let full = build_svg_font(
        &svg_options,
        &prepare_svg_font(&svg_options, &source_files).unwrap(),
    );

    let fresh = build_svg_font(
        &svg_options,
        &prepare_svg_font_incremental(&svg_options, &source_files, &mut cache).unwrap(),
    );
    assert_eq!(
        full, fresh,
        "incremental build with an empty cache must match the full build"
    );
    assert_eq!(
        cache.entries.len(),
        source_files.len(),
        "every glyph should be cached after the first incremental build"
    );

    // A second pass with no content change must serve entirely from the cache and still match.
    let reused = build_svg_font(
        &svg_options,
        &prepare_svg_font_incremental(&svg_options, &source_files, &mut cache).unwrap(),
    );
    assert_eq!(
        full, reused,
        "a cache-reuse build must match the full build"
    );

    let parse_count = cache.parse_count;
    source_files[0].path.push_str("-renamed");
    source_files[0].glyph_name = "renamed".to_string();
    let renamed = build_svg_font(
        &svg_options,
        &prepare_svg_font_incremental(&svg_options, &source_files, &mut cache).unwrap(),
    );
    let renamed_full = build_svg_font(
        &svg_options,
        &prepare_svg_font(&svg_options, &source_files).unwrap(),
    );
    assert_eq!(renamed_full, renamed);
    assert_eq!(
        cache.parse_count, parse_count,
        "identical SVG bytes under a new path should reuse cached geometry"
    );
}

#[test]
fn incremental_prepare_prunes_stale_content_hash_entries() {
    let dir = icons_root().join("cleanicons");
    let mut resolved = input::resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: svg_files(&dir),
        html: Some(false),
        font_name: Some("iconfont".to_string()),
        ligature: Some(false),
        ..Default::default()
    })
    .unwrap();
    let mut source_files = load_source_files(&resolved.files);
    input::finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    let svg_options = svg_options_from_options(&resolved);

    let mut cache = GlyphCache::default();
    prepare_svg_font_incremental(&svg_options, &source_files, &mut cache).unwrap();

    let removed = source_files.pop().expect("fixture should contain files");
    let removed_hash = source_content_hash(&removed.contents);
    prepare_svg_font_incremental(&svg_options, &source_files, &mut cache).unwrap();

    assert!(
        !cache.by_content_hash.contains_key(&removed_hash),
        "removed glyph geometry should not stay in the content-addressed cache"
    );
}

fn winding_fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/svg/fixtures/winding")
}

/// Run a single winding fixture through the real pipeline and return the glyph's serialized path.
fn winding_glyph_path_data(file: &str) -> String {
    let fixture = winding_fixtures_dir()
        .join(file)
        .to_string_lossy()
        .into_owned();
    let mut resolved = input::resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(false),
        html: Some(false),
        dest: "artifacts".to_string(),
        files: vec![fixture],
        font_name: Some("winding".to_string()),
        ..Default::default()
    })
    .unwrap();
    resolved.types = vec![crate::FontType::Svg];
    let source_files = load_source_files(&resolved.files);
    input::finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    let svg_options = svg_options_from_options(&resolved);
    let prepared = prepare_svg_font(&svg_options, &source_files).unwrap();
    prepared.processed_glyphs[0].path_data.to_string()
}

/// Signed-area sign of each subpath in serialized path data (`M`/`L`/`Q`/`C`/`Z`, absolute coords),
/// using on-curve endpoints — enough to read winding direction.
fn subpath_signs(path_data: &str) -> Vec<i8> {
    let t: Vec<&str> = path_data.split_whitespace().collect();
    let num = |s: &str| s.parse::<f64>().expect("path coordinate should parse");
    let poly_sign = |pts: &[(f64, f64)]| -> i8 {
        let mut a = 0.0;
        for k in 0..pts.len() {
            let (x1, y1) = pts[k];
            let (x2, y2) = pts[(k + 1) % pts.len()];
            a += x1 * y2 - x2 * y1;
        }
        if a >= 0.0 {
            return 1;
        }
        -1
    };
    let mut signs = Vec::new();
    let mut pts: Vec<(f64, f64)> = Vec::new();
    let mut i = 0;
    while i < t.len() {
        match t[i] {
            "M" => {
                if pts.len() >= 3 {
                    signs.push(poly_sign(&pts));
                }
                pts = vec![(num(t[i + 1]), num(t[i + 2]))];
                i += 3;
            }
            "L" => {
                pts.push((num(t[i + 1]), num(t[i + 2])));
                i += 3;
            }
            "Q" => {
                pts.push((num(t[i + 3]), num(t[i + 4])));
                i += 5;
            }
            "C" => {
                pts.push((num(t[i + 5]), num(t[i + 6])));
                i += 7;
            }
            _ => i += 1,
        }
    }
    if pts.len() >= 3 {
        signs.push(poly_sign(&pts));
    }
    signs
}

// Deliberate geometry-only heuristic: a fully contained contour becomes a knockout *regardless of
// fill-rule*. `contained-knockout.svg` uses default (nonzero) fill with no `fill-rule` set, so this
// pins that a same/default-fill nested foreground is intentionally converted to a hole for
// monochrome icon-font output — not honoring nonzero's "fill the interior" semantics. See the
// module docs in `winding.rs` for why this trade-off is chosen.
#[test]
fn winding_knocks_out_a_contained_contour() {
    let signs = subpath_signs(&winding_glyph_path_data("contained-knockout.svg"));
    assert_eq!(signs.len(), 2, "circle + one contained square");
    assert_ne!(
        signs[0], signs[1],
        "the contained square must be wound opposite (a knockout hole)"
    );
}

// Overlapping shapes where neither contains the other must stay same-wound (union) — not punched
// into a hole. Guards against the padlock-shackle-on-body class of false positive.
#[test]
fn winding_leaves_overlapping_contours_unioned() {
    let signs = subpath_signs(&winding_glyph_path_data("overlap-union.svg"));
    assert_eq!(signs.len(), 2, "two overlapping, non-nested rectangles");
    assert_eq!(
        signs[0], signs[1],
        "overlapping non-nested contours stay same-wound (union)"
    );
}

// The spec-aligned case: an `evenodd` path with a same-wound inner subpath — containment turns the
// inner into the hole the fill rule intends.
#[test]
fn winding_holes_an_evenodd_inner_subpath() {
    let signs = subpath_signs(&winding_glyph_path_data("evenodd-hole.svg"));
    assert_eq!(signs.len(), 2, "outer + inner square subpaths");
    assert_ne!(
        signs[0], signs[1],
        "the inner even-odd subpath must become a hole"
    );
}

// Curve containment (contours are flattened to decide nesting): a smaller circle clearly nested in a
// larger one must become a ring/hole.
#[test]
fn winding_holes_a_nested_curved_contour() {
    let signs = subpath_signs(&winding_glyph_path_data("nested-circles.svg"));
    assert_eq!(signs.len(), 2, "outer + inner circle");
    assert_ne!(
        signs[0], signs[1],
        "the contained inner circle must become a hole"
    );
}
#[test]
fn oxvg_path_conversion_matches_kurbo_svg_parser() {
    use kurbo::{BezPath, PathEl, Point};
    use oxvg_path::parser::Parse as _;

    let cases = [
        (
            "absolute-relative-implicit-lines",
            "M-0 1.125 2.375 3.625m4.125-5.125 6.125 7.125L20.25 21.375 22.5 23.625l2.125-3.25 4.5 5.625H30.125 31.25h2.375-1.125V40.5 41.625v2.75-1.5",
            true,
        ),
        (
            "curves-and-cross-family-controls",
            "M10 20C11.125 12.25 13.375 14.5 15.625 16.75 17.875 18.125 19.25 20.375 21.5 22.625S23.75 24.875 25 26.125s1.25 2.375 3.5 4.625Q31.25 32.375 33.5 34.625 35.75 36.875 38 39.125T40.25 41.375t2.5 3.625S47 48.125 49.25 50.375T52.5 53.625c1.125 2.25 3.375 4.5 5.625 6.75q1.25 2.375 3.5 4.625",
            false,
        ),
        (
            "arcs-flags-rotation-relative-degenerate",
            "M1.125 2.25A10.375 20.5 37.25 0 1 30.625 40.75a-12.875 8.25-45.5 1 0 15.125-9.375A0 5 0 0 1 50.25 60.375a5 0 90 1 1-2.125 3.25A4 6-0 0 0 48.125 63.625",
            false,
        ),
        (
            "negative-zero-and-three-decimals",
            "M-0-0L.001-.002l-.003.004H-0v-0C.125-.25.375-.5.625-.75Q.875-.999 1.001-1.125",
            false,
        ),
        (
            "subpaths-close-and-drawing",
            "M1 2Q2 3 3 4ZS5 6 7 8T9 10M20 21l2 3zL4 5Z",
            false,
        ),
    ];

    for (name, source, nest_implicit) in cases {
        let mut path = oxvg_path::Path::parse_string(source).unwrap();
        if nest_implicit {
            let command = path
                .0
                .iter_mut()
                .find(|command| command.is_implicit())
                .unwrap();
            *command = oxvg_path::command::Data::Implicit(Box::new(command.clone()));
        }
        let direct = super::geometry::bezpath_from_oxvg_path(&path);
        let parsed = BezPath::from_svg(&path.to_string()).unwrap();
        assert_eq!(
            direct.elements().len(),
            parsed.elements().len(),
            "{name}: element count"
        );

        let assert_point = |index: usize, field: &str, direct: Point, parsed: Point| {
            assert_eq!(
                [direct.x.to_bits(), direct.y.to_bits()],
                [parsed.x.to_bits(), parsed.y.to_bits()],
                "{name}: element {index} {field}"
            );
        };
        for (index, (direct, parsed)) in direct.elements().iter().zip(parsed.elements()).enumerate()
        {
            match (*direct, *parsed) {
                (PathEl::MoveTo(a), PathEl::MoveTo(b)) | (PathEl::LineTo(a), PathEl::LineTo(b)) => {
                    assert_point(index, "point", a, b);
                }
                (PathEl::QuadTo(a1, a2), PathEl::QuadTo(b1, b2)) => {
                    assert_point(index, "control", a1, b1);
                    assert_point(index, "point", a2, b2);
                }
                (PathEl::CurveTo(a1, a2, a3), PathEl::CurveTo(b1, b2, b3)) => {
                    assert_point(index, "control1", a1, b1);
                    assert_point(index, "control2", a2, b2);
                    assert_point(index, "point", a3, b3);
                }
                (PathEl::ClosePath, PathEl::ClosePath) => {}
                (direct, parsed) => {
                    panic!("{name}: element {index} variant differs: {direct:?} != {parsed:?}")
                }
            }
        }
    }
}

#[test]
fn quadratic_tiny_paths_are_rounded_and_fully_hashed() {
    use kurbo::{PathEl, Point};
    use usvg::tiny_skia_path::PathBuilder;

    let make_path = |control_x, end_y| {
        let mut builder = PathBuilder::new();
        builder.move_to(0.4, 0.6);
        builder.quad_to(control_x, 2.6, 3.4, end_y);
        builder.finish().unwrap()
    };
    let path = super::geometry::rounded_bezpath_from_tiny_paths(&[make_path(1.4, 4.6)], 1.0);

    assert_eq!(
        path.elements(),
        &[
            PathEl::MoveTo(Point::new(0.0, 1.0)),
            PathEl::QuadTo(Point::new(1.0, 3.0), Point::new(3.0, 5.0)),
        ]
    );
    assert_ne!(
        super::geometry::bezpath_hash(&path),
        super::geometry::bezpath_hash(&super::geometry::rounded_bezpath_from_tiny_paths(
            &[make_path(2.4, 4.6)],
            1.0,
        ))
    );
    assert_ne!(
        super::geometry::bezpath_hash(&path),
        super::geometry::bezpath_hash(&super::geometry::rounded_bezpath_from_tiny_paths(
            &[make_path(1.4, 5.6)],
            1.0,
        ))
    );
}

fn variant_svg(path: &str, name: &str, width: u32, height: u32, shape_width: u32) -> LoadedSvgFile {
    LoadedSvgFile {
        contents: format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><rect width=\"{shape_width}\" height=\"{height}\"/></svg>"
        )
        .into(),
        glyph_name: name.to_owned(),
        path: path.to_owned(),
    }
}

fn resolved_family(
    variants: Vec<Vec<LoadedSvgFile>>,
    behavior: MissingGlyphBehavior,
    fallback_index: Option<usize>,
    variant_names: &[&str],
) -> (VariantFamilySources, BTreeMap<String, u32>) {
    let (mut family, codepoints) =
        input::build_variant_family_sources(variants, &BTreeMap::new(), 0xe001).unwrap();
    input::resolve_missing_glyphs(&mut family, behavior, fallback_index, variant_names).unwrap();
    (family, codepoints)
}

fn prepare_family(
    family: &VariantFamilySources,
    codepoints: BTreeMap<String, u32>,
    configure: impl FnOnce(&mut ResolvedGenerateWebfontsOptions),
) -> PreparedVariantFamily {
    let mut options = input::resolve_generate_webfonts_options(GenerateWebfontsOptions {
        dest: "artifacts".to_owned(),
        files: vec!["unused.svg".to_owned()],
        types: Some(vec![FontType::Ttf]),
        write_files: Some(false),
        ..Default::default()
    })
    .unwrap();
    options.codepoints = codepoints;
    configure(&mut options);

    prepare_variant_svg_family(&svg_options_from_options(&options), family).unwrap()
}

#[test]
fn variant_family_uses_shared_metrics_and_logical_advances() {
    let (family, codepoints) = resolved_family(
        vec![
            vec![
                variant_svg("small/a.svg", "a", 10, 20, 10),
                variant_svg("small/b.svg", "b", 4, 10, 4),
            ],
            vec![variant_svg("large/a.svg", "a", 30, 10, 30)],
        ],
        MissingGlyphBehavior::Blank,
        None,
        &["small", "large"],
    );
    let prepared = prepare_family(&family, codepoints.clone(), |_| {});

    assert_eq!(
        (prepared.ascent, prepared.descent, prepared.font_height),
        (20.0, 0.0, 20.0)
    );
    assert_eq!(prepared.glyphs[0].advance_width, 20.0);
    assert_eq!(prepared.glyphs[1].advance_width, 8.0);
    assert_eq!(prepared.glyphs[0].outlines[0].as_ref().unwrap().width, 20.0);
    assert_eq!(prepared.glyphs[0].outlines[1].as_ref().unwrap().width, 20.0);
    assert_eq!(prepared.glyphs[1].outlines[0].as_ref().unwrap().width, 8.0);
    assert!(prepared.glyphs[1].outlines[1].is_none());

    let fixed = prepare_family(&family, codepoints, |options| {
        options.fixed_width = Some(true);
    });
    assert!(fixed.glyphs.iter().all(|glyph| glyph.advance_width == 20.0));
    assert!(fixed.glyphs.iter().all(|glyph| {
        glyph
            .outlines
            .iter()
            .flatten()
            .all(|outline| outline.width == 20.0)
    }));
}

#[test]
fn variant_family_applies_fallback_advances_without_overwriting_direct_outlines() {
    let (family, codepoints) = resolved_family(
        vec![
            vec![
                variant_svg("small/a.svg", "a", 10, 20, 10),
                variant_svg("small/b.svg", "b", 4, 10, 4),
            ],
            vec![variant_svg("large/a.svg", "a", 30, 10, 30)],
        ],
        MissingGlyphBehavior::Fallback,
        Some(0),
        &["small", "large"],
    );
    let prepared = prepare_family(&family, codepoints, |_| {});

    let b = &prepared.glyphs[1];
    assert_eq!(b.advance_width, 8.0);
    assert!(b.outlines.iter().all(Option::is_some));
    assert_eq!(b.outlines[0].as_ref().unwrap().width, 8.0);
    assert_eq!(b.outlines[1].as_ref().unwrap().width, 8.0);
    assert_eq!(
        b.outlines[0].as_ref().unwrap().ttf_path,
        b.outlines[1].as_ref().unwrap().ttf_path
    );
}

#[test]
fn variant_family_metrics_are_order_independent_and_center_with_explicit_overrides() {
    let variants = vec![
        vec![variant_svg("small/a.svg", "a", 20, 10, 10)],
        vec![variant_svg("large/a.svg", "a", 30, 20, 30)],
    ];
    let (family, codepoints) = resolved_family(
        variants.clone(),
        MissingGlyphBehavior::Blank,
        None,
        &["small", "large"],
    );
    let (reversed, reversed_codepoints) = resolved_family(
        variants.into_iter().rev().collect(),
        MissingGlyphBehavior::Blank,
        None,
        &["large", "small"],
    );
    let natural = prepare_family(&family, codepoints.clone(), |_| {});
    let reversed_natural = prepare_family(&reversed, reversed_codepoints.clone(), |_| {});
    assert_eq!(
        (natural.ascent, natural.descent, natural.font_height),
        (
            reversed_natural.ascent,
            reversed_natural.descent,
            reversed_natural.font_height
        )
    );
    assert_eq!(
        natural.glyphs[0].advance_width,
        reversed_natural.glyphs[0].advance_width
    );

    let configure = |options: &mut ResolvedGenerateWebfontsOptions| {
        options.ascent = Some(80.0);
        options.descent = Some(20.0);
        options.font_height = Some(100.0);
        options.normalize = false;
        options.center_horizontally = Some(true);
        options.center_vertically = Some(true);
    };
    let prepared = prepare_family(&family, codepoints, configure);
    let reversed = prepare_family(&reversed, reversed_codepoints, configure);

    assert_eq!(
        (prepared.ascent, prepared.descent, prepared.font_height),
        (80.0, 20.0, 100.0)
    );
    assert_eq!(prepared.glyphs[0].advance_width, 150.0);
    assert_eq!(
        prepared.glyphs[0].advance_width,
        reversed.glyphs[0].advance_width
    );
    for outline in prepared.glyphs[0].outlines.iter().flatten() {
        let bounds = outline.ttf_path.as_ref().unwrap().bounding_box();
        assert!((bounds.x0 + bounds.x1 - outline.width).abs() < 0.001);
        assert!((bounds.y0 + bounds.y1 - 60.0).abs() < 0.001);
    }
}

fn oracle_glyphs(prepared: &PreparedVariantFamily, variant_index: usize) -> Vec<ProcessedGlyph> {
    prepared
        .glyphs
        .iter()
        .enumerate()
        .map(|(index, glyph)| {
            glyph.outlines[variant_index]
                .clone()
                .unwrap_or_else(|| ProcessedGlyph {
                    codepoint: glyph.codepoint,
                    height: 0.0,
                    index,
                    name: glyph.name.clone(),
                    path_data: Arc::from(""),
                    ttf_path: None,
                    ttf_path_hash: None,
                    width: glyph.advance_width,
                })
        })
        .collect()
}

#[test]
fn static_variant_oracles_share_metrics_codepoints_and_advances() {
    let (family, codepoints) = resolved_family(
        vec![
            vec![
                variant_svg("small/a.svg", "a", 10, 20, 10),
                variant_svg("small/b.svg", "b", 4, 10, 4),
            ],
            vec![variant_svg("large/a.svg", "a", 30, 10, 30)],
        ],
        MissingGlyphBehavior::Blank,
        None,
        &["small", "large"],
    );
    let prepared = prepare_family(&family, codepoints, |_| {});
    let build = |prepared: &PreparedVariantFamily, variant_index, weight| {
        crate::sfnt::build(
            crate::sfnt::TtfOptions {
                ascent: Some(prepared.ascent),
                copyright: None,
                descent: Some(prepared.descent),
                description: None,
                font_height: Some(prepared.font_height),
                font_name: "variant-oracle",
                font_style: None,
                font_weight: Some(weight),
                ligature: false,
                manufacturer_url: None,
                ts: Some(0),
                version: None,
            },
            &oracle_glyphs(&prepared, variant_index),
            None,
        )
        .unwrap()
    };
    let small_tables = build(&prepared, 0, "300");
    let large_tables = build(&prepared, 1, "700");
    let small = FontRef::new(small_tables.ttf()).unwrap();
    let large = FontRef::new(large_tables.ttf()).unwrap();

    for (font, weight) in [(&small, 300), (&large, 700)] {
        assert_eq!(font.head().unwrap().units_per_em(), 20);
        assert_eq!(font.hhea().unwrap().ascender(), 20.into());
        assert_eq!(font.hhea().unwrap().descender(), 0.into());
        assert_eq!(font.os2().unwrap().us_weight_class(), weight);
        assert!(font.cmap().unwrap().map_codepoint(0xe001_u32).is_some());
        assert_eq!(font.hmtx().unwrap().h_metrics()[1].advance(), 20);
        assert_eq!(font.hmtx().unwrap().h_metrics()[2].advance(), 8);
    }
    let glyph_bounds = |font: &FontRef<'_>, codepoint| {
        let glyph_id = font.cmap().unwrap().map_codepoint(codepoint).unwrap();
        let glyf = font.glyf().unwrap();
        match font.loca(None).unwrap().get_glyf(glyph_id, &glyf).unwrap() {
            Some(Glyph::Simple(glyph)) => {
                Some((glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max()))
            }
            None => None,
            Some(Glyph::Composite(_)) => panic!("variant oracle should use simple glyphs"),
        }
    };
    assert_eq!(glyph_bounds(&small, 0xe001_u32), Some((0, 0, 10, 20)));
    assert_eq!(glyph_bounds(&large, 0xe001_u32), Some((0, 0, 20, 7)));
    assert!(glyph_bounds(&small, 0xe002_u32).is_some());
    assert_eq!(glyph_bounds(&large, 0xe002_u32), None);

    let (fallback_family, fallback_codepoints) = resolved_family(
        family.variants.clone(),
        MissingGlyphBehavior::Fallback,
        Some(0),
        &["small", "large"],
    );
    let fallback = prepare_family(&fallback_family, fallback_codepoints, |_| {});
    let fallback_large_tables = build(&fallback, 1, "700");
    let fallback_large = FontRef::new(fallback_large_tables.ttf()).unwrap();
    assert_eq!(
        glyph_bounds(&fallback_large, 0xe002_u32),
        glyph_bounds(&small, 0xe002_u32)
    );

    assert_ne!(
        small.table_data(Tag::new(b"glyf")).unwrap().as_bytes(),
        large.table_data(Tag::new(b"glyf")).unwrap().as_bytes()
    );
    assert_eq!(
        GlyphId::new(1),
        small.cmap().unwrap().map_codepoint(0xe001_u32).unwrap()
    );
}
