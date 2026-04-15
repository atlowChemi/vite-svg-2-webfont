// Integration tests exercise the pure Rust API and CLI. They cannot link against
// the NAPI feature because the test binary is not a Node.js addon.
#![cfg(not(feature = "napi"))]

use std::collections::HashMap;
use std::path::Path;

use webfont_generator::{FontType, GenerateWebfontsOptions};

fn fixture_files() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/svg/fixtures/icons/cleanicons");
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .expect("fixture dir should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("svg"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    files.sort();
    files
}

fn temp_dest(prefix: &str) -> String {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir()
        .join(format!("{prefix}-{unique}"))
        .to_string_lossy()
        .into_owned()
}

// --- generate_sync tests ---

#[test]
fn generate_sync_produces_all_default_font_types() {
    let dest = temp_dest("gen-sync-defaults");
    let result = webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest: dest.clone(),
            files: fixture_files(),
            write_files: Some(false),
            ..Default::default()
        },
        None,
    )
    .expect("generate_sync should succeed");

    // Default types: eot, woff, woff2
    assert!(result.eot_bytes().is_some(), "should generate EOT");
    assert!(result.woff_bytes().is_some(), "should generate WOFF");
    assert!(result.woff2_bytes().is_some(), "should generate WOFF2");
    // SVG and TTF are not in the default types
    assert!(
        result.svg_string().is_none(),
        "should not generate SVG by default"
    );
    assert!(
        result.ttf_bytes().is_none(),
        "should not generate TTF by default"
    );
}

#[test]
fn generate_sync_produces_requested_types_only() {
    let dest = temp_dest("gen-sync-types");
    let result = webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest,
            files: fixture_files(),
            types: Some(vec![FontType::Svg, FontType::Ttf]),
            write_files: Some(false),
            ..Default::default()
        },
        None,
    )
    .expect("generate_sync should succeed");

    assert!(result.svg_string().is_some(), "should generate SVG");
    assert!(result.ttf_bytes().is_some(), "should generate TTF");
    assert!(result.eot_bytes().is_none(), "should not generate EOT");
    assert!(result.woff_bytes().is_none(), "should not generate WOFF");
    assert!(result.woff2_bytes().is_none(), "should not generate WOFF2");
}

#[test]
fn generate_sync_writes_files_to_disk() {
    let dest = temp_dest("gen-sync-write");
    let font_name = "test-icons";
    let result = webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest: dest.clone(),
            files: fixture_files(),
            font_name: Some(font_name.to_owned()),
            types: Some(vec![FontType::Woff2]),
            css: Some(true),
            write_files: Some(true),
            ..Default::default()
        },
        None,
    )
    .expect("generate_sync should succeed");

    assert!(result.woff2_bytes().is_some());
    assert!(
        Path::new(&dest).join(format!("{font_name}.woff2")).exists(),
        "WOFF2 file should be written"
    );
    assert!(
        Path::new(&dest).join(format!("{font_name}.css")).exists(),
        "CSS file should be written"
    );

    // Clean up
    let _ = std::fs::remove_dir_all(&dest);
}

#[test]
fn generate_sync_generates_valid_css() {
    let dest = temp_dest("gen-sync-css");
    let result = webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest,
            files: fixture_files(),
            font_name: Some("my-icons".to_owned()),
            types: Some(vec![FontType::Woff2]),
            css: Some(true),
            write_files: Some(false),
            ..Default::default()
        },
        None,
    )
    .expect("generate_sync should succeed");

    let css = result
        .generate_css_pure(None)
        .expect("CSS generation should succeed");

    assert!(css.contains("@font-face"), "CSS should contain @font-face");
    assert!(
        css.contains("font-family: \"my-icons\""),
        "CSS should use the configured font name"
    );
    assert!(
        css.contains("format(\"woff2\")"),
        "CSS should reference woff2 format"
    );
}

#[test]
fn generate_sync_generates_html_when_requested() {
    let dest = temp_dest("gen-sync-html");
    let result = webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest,
            files: fixture_files(),
            types: Some(vec![FontType::Woff2]),
            html: Some(true),
            write_files: Some(false),
            ..Default::default()
        },
        None,
    )
    .expect("generate_sync should succeed");

    let html = result
        .generate_html_pure(None)
        .expect("HTML generation should succeed");

    assert!(
        html.contains("<!DOCTYPE html>") || html.contains("<html"),
        "should produce HTML"
    );
    assert!(
        html.contains("@font-face"),
        "HTML should embed CSS with @font-face"
    );
}

#[test]
fn generate_sync_applies_rename_callback() {
    let dest = temp_dest("gen-sync-rename");
    let result = webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest,
            files: fixture_files(),
            types: Some(vec![FontType::Svg]),
            css: Some(true),
            write_files: Some(false),
            ..Default::default()
        },
        Some(Box::new(|name: &str| format!("prefix-{name}"))),
    )
    .expect("generate_sync should succeed");

    let css = result
        .generate_css_pure(None)
        .expect("CSS generation should succeed");

    assert!(
        css.contains("prefix-"),
        "renamed glyphs should appear in CSS"
    );
}

#[test]
fn generate_sync_rejects_empty_dest() {
    match webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest: String::new(),
            files: fixture_files(),
            ..Default::default()
        },
        None,
    ) {
        Err(err) => {
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
            assert!(err.to_string().contains("dest"));
        }
        Ok(_) => panic!("should fail with empty dest"),
    }
}

#[test]
fn generate_sync_rejects_empty_files() {
    match webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest: "output".to_owned(),
            files: vec![],
            ..Default::default()
        },
        None,
    ) {
        Err(err) => {
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
            assert!(err.to_string().contains("files"));
        }
        Ok(_) => panic!("should fail with empty files"),
    }
}

#[test]
fn generate_sync_with_explicit_codepoints() {
    let dest = temp_dest("gen-sync-codepoints");
    let files = fixture_files();
    let first_glyph = Path::new(&files[0])
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let result = webfont_generator::generate_sync(
        GenerateWebfontsOptions {
            dest,
            files,
            types: Some(vec![FontType::Svg]),
            codepoints: Some(HashMap::from([(first_glyph.clone(), 0xE900)])),
            css: Some(true),
            write_files: Some(false),
            ..Default::default()
        },
        None,
    )
    .expect("generate_sync should succeed");

    let css = result
        .generate_css_pure(None)
        .expect("CSS generation should succeed");

    assert!(
        css.contains("e900"),
        "CSS should contain the explicit codepoint"
    );
}

// --- async generate tests ---

#[tokio::test]
async fn generate_async_produces_fonts() {
    let dest = temp_dest("gen-async");
    let result = webfont_generator::generate(
        GenerateWebfontsOptions {
            dest,
            files: fixture_files(),
            types: Some(vec![FontType::Woff2, FontType::Svg]),
            write_files: Some(false),
            ..Default::default()
        },
        None,
    )
    .await
    .expect("async generate should succeed");

    assert!(result.woff2_bytes().is_some());
    assert!(result.svg_string().is_some());
}

// --- CLI tests ---

#[cfg(feature = "cli")]
mod cli {
    use std::process::Command;

    fn cli_bin() -> Command {
        Command::new(env!("CARGO_BIN_EXE_webfont-generator"))
    }

    fn fixture_dir() -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/svg/fixtures/icons/cleanicons")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn help_flag_succeeds() {
        let output = cli_bin().arg("--help").output().expect("should execute");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Generate webfonts from SVG icons"));
    }

    #[test]
    fn version_flag_succeeds() {
        let output = cli_bin().arg("--version").output().expect("should execute");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("webfont-generator"));
    }

    #[test]
    fn generates_fonts_from_directory() {
        let dest = super::temp_dest("cli-dir");
        let output = cli_bin()
            .args(["--dest", &dest, "--types", "woff2", &fixture_dir()])
            .output()
            .expect("should execute");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "CLI should succeed. stdout: {stdout}, stderr: {stderr}"
        );
        assert!(stdout.contains("WOFF2"), "should report generated WOFF2");
        assert!(
            std::path::Path::new(&dest).join("iconfont.woff2").exists(),
            "WOFF2 file should be written"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn fails_with_no_files() {
        let output = cli_bin()
            .args(["--dest", "/tmp/empty", "/nonexistent/path"])
            .output()
            .expect("should execute");

        assert!(!output.status.success());
    }

    #[test]
    fn generates_css_and_html() {
        let dest = super::temp_dest("cli-css-html");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "woff2",
                "--html",
                "--font-name",
                "test-font",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(output.status.success(), "CLI should succeed");
        assert!(
            std::path::Path::new(&dest).join("test-font.css").exists(),
            "CSS file should be written"
        );
        assert!(
            std::path::Path::new(&dest).join("test-font.html").exists(),
            "HTML file should be written"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn no_css_suppresses_css_output() {
        let dest = super::temp_dest("cli-no-css");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "woff2",
                "--no-css",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(output.status.success(), "CLI should succeed");
        assert!(
            std::path::Path::new(&dest).join("iconfont.woff2").exists(),
            "font file should be written"
        );
        assert!(
            !std::path::Path::new(&dest).join("iconfont.css").exists(),
            "CSS file should NOT be written with --no-css"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn no_html_suppresses_html_even_with_html_flag() {
        let dest = super::temp_dest("cli-no-html");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "woff2",
                "--html",
                "--no-html",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(output.status.success(), "CLI should succeed");
        assert!(
            !std::path::Path::new(&dest).join("iconfont.html").exists(),
            "HTML file should NOT be written when --no-html overrides --html"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn no_write_suppresses_all_file_output() {
        let dest = super::temp_dest("cli-no-write");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "woff2",
                "--no-write",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(output.status.success(), "CLI should succeed");
        assert!(
            !std::path::Path::new(&dest).exists(),
            "dest directory should not be created with --no-write"
        );
    }

    #[test]
    fn rejects_invalid_font_type() {
        let output = cli_bin()
            .args(["--dest", "/tmp/test", "--types", "wof22", &fixture_dir()])
            .output()
            .expect("should execute");

        assert!(!output.status.success(), "should fail with invalid type");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("invalid value"),
            "should report invalid value"
        );
    }

    #[test]
    fn invalid_start_codepoint_uses_default() {
        let dest = super::temp_dest("cli-bad-codepoint");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "woff2",
                "--no-css",
                "--start-codepoint",
                "not-a-number",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(
            output.status.success(),
            "should succeed with invalid codepoint (falls back to default)"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn ligature_no_ligature_last_flag_wins() {
        let dest = super::temp_dest("cli-ligature-precedence");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "svg",
                "--no-css",
                "--ligature",
                "--no-ligature",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(output.status.success(), "CLI should succeed");

        // With ligatures enabled, each glyph gets two <glyph> entries (codepoint +
        // ligature string). Without ligatures, only the codepoint entry exists.
        // The fixture dir has 10 SVGs, so we expect exactly 10 glyphs (no ligatures).
        let svg =
            std::fs::read_to_string(std::path::Path::new(&dest).join("iconfont.svg")).unwrap();
        let glyph_count = svg.matches("<glyph ").count();
        assert_eq!(
            glyph_count, 10,
            "should have exactly 10 <glyph> entries (no ligature duplicates) when --no-ligature wins"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn no_css_then_css_restores_css_output() {
        let dest = super::temp_dest("cli-no-css-css");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "woff2",
                "--no-css",
                "--css",
                "--font-name",
                "precedence-test",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(output.status.success(), "CLI should succeed");
        assert!(
            std::path::Path::new(&dest)
                .join("precedence-test.css")
                .exists(),
            "CSS should be generated when --css overrides earlier --no-css"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn no_write_then_write_restores_file_output() {
        let dest = super::temp_dest("cli-no-write-write");
        let output = cli_bin()
            .args([
                "--dest",
                &dest,
                "--types",
                "woff2",
                "--no-css",
                "--no-write",
                "--write",
                &fixture_dir(),
            ])
            .output()
            .expect("should execute");

        assert!(output.status.success(), "CLI should succeed");
        assert!(
            std::path::Path::new(&dest).join("iconfont.woff2").exists(),
            "font should be written when --write overrides earlier --no-write"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }
}
