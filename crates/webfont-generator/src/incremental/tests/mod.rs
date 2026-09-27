use std::path::Path;

use super::RegenerationState;
use crate::input::{
    LoadedSvgFile, finalize_generate_webfonts_options, resolve_generate_webfonts_options,
};
use crate::pipeline::generate_webfonts_sync;
use crate::result::GenerateWebfontsResult;
use crate::types::{FontType, GlyphChange};
use crate::{FormatOptions, GenerateWebfontsOptions, TtfFormatOptions};

mod caching;
mod output;
mod regeneration;
mod rendering;

const D1: &str = "M2 2 L22 2 L22 22 Z";
const D2: &str = "M2 2 L22 2 L12 22 Z";
const D3: &str = "M4 4 L20 4 L20 20 L4 20 Z";
const D_CHANGED: &str = "M0 0 L24 0 L24 24 Z";
const TEST_TTF_TIMESTAMP: i64 = 1_700_000_000;

fn with_regeneration_state<T>(
    result: &GenerateWebfontsResult,
    read: impl FnOnce(&RegenerationState) -> T,
) -> T {
    let state = result.regeneration_state.lock().unwrap();
    read(state.as_ref().unwrap())
}

fn stable_format_options(with_metadata: bool) -> FormatOptions {
    FormatOptions {
        ttf: Some(TtfFormatOptions {
            copyright: with_metadata.then(|| "Copyright 2026".to_owned()),
            description: with_metadata.then(|| "Incremental test font".to_owned()),
            ts: Some(TEST_TTF_TIMESTAMP),
            url: with_metadata.then(|| "https://example.com".to_owned()),
            version: with_metadata.then(|| "1.0".to_owned()),
        }),
        ..Default::default()
    }
}

fn temp_dir() -> std::path::PathBuf {
    // Process id + a monotonic counter so parallel tests never collide on the same dir.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("recalc-ut-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_icon(dir: &Path, name: &str, d: &str) -> String {
    write_icon_with_viewbox(dir, name, 24, 24, d)
}

fn write_icon_with_viewbox(dir: &Path, name: &str, width: u32, height: u32, d: &str) -> String {
    let path = dir.join(format!("{name}.svg"));
    std::fs::write(
        &path,
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\"><path d=\"{d}\"/></svg>"),
    )
    .unwrap();
    path.to_string_lossy().into_owned()
}

fn load(paths: &[String]) -> Vec<LoadedSvgFile> {
    paths
        .iter()
        .map(|path| LoadedSvgFile {
            contents: std::fs::read_to_string(path).unwrap().into(),
            glyph_name: Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned(),
            path: path.clone(),
        })
        .collect()
}

fn generate(paths: Vec<String>, incremental: bool) -> GenerateWebfontsResult {
    generate_with_ligatures(paths, incremental, false, false)
}

fn generate_with_ligatures(
    paths: Vec<String>,
    incremental: bool,
    ligature: bool,
    with_metadata: bool,
) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_owned(),
        files: paths,
        html: Some(false),
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options(with_metadata)),
        ligature: Some(ligature),
        incremental: Some(incremental),
        // These tests assert in-memory parity; don't touch the disk on regenerate.
        write_files: Some(false),
        types: Some(vec![
            FontType::Svg,
            FontType::Ttf,
            FontType::Eot,
            FontType::Woff,
            FontType::Woff2,
        ]),
        ..Default::default()
    })
    .unwrap();
    let source_files = load(&resolved.files);
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    generate_webfonts_sync(resolved, source_files).unwrap()
}

fn generate_with_css(paths: Vec<String>, incremental: bool) -> GenerateWebfontsResult {
    let mut resolved = resolve_generate_webfonts_options(GenerateWebfontsOptions {
        css: Some(true),
        dest: "artifacts".to_owned(),
        files: paths,
        html: Some(false),
        font_name: Some("rc".to_owned()),
        format_options: Some(stable_format_options(false)),
        ligature: Some(false),
        incremental: Some(incremental),
        // These tests assert in-memory parity; don't touch the disk on regenerate.
        write_files: Some(false),
        types: Some(vec![FontType::Woff2]),
        ..Default::default()
    })
    .unwrap();
    let source_files = load(&resolved.files);
    finalize_generate_webfonts_options(&mut resolved, &source_files).unwrap();
    generate_webfonts_sync(resolved, source_files).unwrap()
}

fn assert_same(actual: &GenerateWebfontsResult, expected: &GenerateWebfontsResult) {
    assert_eq!(actual.svg_string(), expected.svg_string(), "svg mismatch");
    assert_eq!(actual.ttf_bytes(), expected.ttf_bytes(), "ttf mismatch");
    assert_eq!(actual.eot_bytes(), expected.eot_bytes(), "eot mismatch");
    assert_eq!(actual.woff_bytes(), expected.woff_bytes(), "woff mismatch");
    assert_eq!(
        actual.woff2_bytes(),
        expected.woff2_bytes(),
        "woff2 mismatch"
    );
}
