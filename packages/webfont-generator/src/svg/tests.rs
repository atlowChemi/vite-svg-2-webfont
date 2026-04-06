use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{build_svg_font, prepare_svg_font, svg_options_from_options};
use napi::sys::{napi_env, napi_ref, napi_status};

use crate::{
    finalize_generate_webfonts_options, resolve_generate_webfonts_options, FormatOptions,
    GenerateWebfontsOptions, LoadedSvgFile, SvgFormatOptions,
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

#[unsafe(no_mangle)]
extern "C" fn napi_delete_reference(_: napi_env, _: napi_ref) -> napi_status {
    0
}

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
    let mut resolved_options = resolve_generate_webfonts_options(options)
        .unwrap_or_else(|error| panic!("native options should resolve: {error}"));
    let source_files = load_source_files(&resolved_options.files);
    finalize_generate_webfonts_options(&mut resolved_options, &source_files)
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
                .unwrap_or_else(|error| panic!("failed to load source SVG '{}': {error}", path)),
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
    let mut resolved_options = resolve_generate_webfonts_options(options.clone())
        .unwrap_or_else(|error| panic!("native options should resolve: {error}"));
    let source_files = load_source_files(&resolved_options.files);
    finalize_generate_webfonts_options(&mut resolved_options, &source_files)
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
    let svg = generate_svg_font(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: vec![icons_root()
            .join("emptyicons/empty.svg")
            .to_string_lossy()
            .into_owned()],
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
