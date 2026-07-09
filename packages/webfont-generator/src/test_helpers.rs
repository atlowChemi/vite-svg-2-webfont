use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{LoadedSvgFile, ResolvedGenerateWebfontsOptions};
use crate::{GenerateWebfontsOptions, resolve_generate_webfonts_options};

pub fn resolve_options(options: GenerateWebfontsOptions) -> ResolvedGenerateWebfontsOptions {
    resolve_generate_webfonts_options(options)
        .unwrap_or_else(|error| panic!("native options should resolve: {error}"))
}

pub fn write_temp_template(prefix: &str, contents: &str) -> String {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{unique}.hbs", std::process::id()));
    fs::write(&path, contents).expect("temporary template should be written");
    path.to_string_lossy().into_owned()
}

pub fn fixture_source_files(options: &ResolvedGenerateWebfontsOptions) -> Vec<LoadedSvgFile> {
    vec![LoadedSvgFile {
        contents: fs::read_to_string(&options.files[0]).expect("fixture should load"),
        glyph_name: Path::new(&options.files[0])
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("add")
            .to_owned(),
        path: options.files[0].clone(),
    }]
}

pub fn webfont_fixture(name: &str) -> String {
    format!(
        "{}/../vite-svg-2-webfont/src/fixtures/webfont-test/svg/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

pub fn fixture_font_tables() -> crate::sfnt::SerializedFontTables {
    let mut options = resolve_options(GenerateWebfontsOptions {
        css: Some(false),
        dest: "artifacts".to_string(),
        files: vec![webfont_fixture("add.svg")],
        html: Some(false),
        font_name: Some("iconfont".to_string()),
        ligature: Some(false),
        ..Default::default()
    });
    let source_files = fixture_source_files(&options);
    crate::finalize_generate_webfonts_options(&mut options, &source_files)
        .expect("expected options to finalize");
    let svg_options = crate::svg_options_from_options(&options);
    let prepared = crate::svg::prepare_svg_font(&svg_options, &source_files)
        .expect("expected svg preparation to succeed");
    crate::ttf::generate_ttf_font_from_glyphs(
        crate::ttf::ttf_options_from_options(&options),
        &prepared.processed_glyphs,
    )
    .expect("expected ttf generation to succeed")
}

pub fn icons_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/svg/fixtures/icons")
        .canonicalize()
        .expect("native icon fixture root should exist")
}

pub fn update_snapshots_enabled() -> bool {
    std::env::var_os("UPDATE_SVG_FIXTURES").is_some_and(|value| value != "0")
}
