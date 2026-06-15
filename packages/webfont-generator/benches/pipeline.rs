use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::Value;
use webfont_generator::bench_support::{
    BenchSvgSource, build_outputs_only, finalize_svg_only, parse_svg_only,
};
use webfont_generator::{
    FontType, FormatOptions, GenerateWebfontsOptions, SvgFormatOptions, TtfFormatOptions,
    Woff2FormatOptions,
};

const SIZES: [usize; 3] = [100, 300, 600];
const BENCH_DEST: &str = "/tmp/webfont-generator-pipeline-bench-out";
const TEST_TTF_TIMESTAMP: i64 = 1_700_000_000;

struct FixtureSet {
    dir: PathBuf,
    paths: Vec<String>,
    sources: Vec<BenchSvgSource>,
}

impl Drop for FixtureSet {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "webfont-generator-pipeline-bench-{prefix}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn path_data(index: usize) -> String {
    let mut d = String::new();
    let start_x = 2.0 + (index % 5) as f64 * 0.35;
    let start_y = 2.0 + (index % 7) as f64 * 0.25;
    d.push_str(&format!("M{start_x:.2} {start_y:.2}"));
    for segment in 0..24 {
        let t = (index + segment) as f64;
        let x1 = 3.0 + ((t * 1.7) % 18.0);
        let y1 = 3.0 + ((t * 2.3) % 18.0);
        let x2 = 3.0 + ((t * 3.1 + 5.0) % 18.0);
        let y2 = 3.0 + ((t * 1.3 + 7.0) % 18.0);
        let x = 3.0 + ((t * 2.9 + 11.0) % 18.0);
        let y = 3.0 + ((t * 3.7 + 13.0) % 18.0);
        d.push_str(&format!(" C{x1:.2} {y1:.2} {x2:.2} {y2:.2} {x:.2} {y:.2}"));
    }
    d.push_str(" Z");
    d
}

fn svg(path_data: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"{path_data}\"/></svg>"
    )
}

fn iconify_json_path(icon_set: &str) -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let direct = root
        .join("node_modules")
        .join("@iconify-json")
        .join(icon_set)
        .join("icons.json");
    if direct.exists() {
        return Some(direct);
    }

    let pnpm = root.join("node_modules/.pnpm");
    let prefix = format!("@iconify-json+{icon_set}@");
    for entry in std::fs::read_dir(pnpm).ok()? {
        let entry = entry.ok()?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.starts_with(&prefix) {
            continue;
        }
        let path = entry
            .path()
            .join("node_modules")
            .join("@iconify-json")
            .join(icon_set)
            .join("icons.json");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn iconify_svgs(size: usize) -> Option<Vec<(String, String)>> {
    let icon_set = std::env::var("BENCH_ICON_SET").unwrap_or_else(|_| "simple-icons".to_owned());
    let path = iconify_json_path(&icon_set)?;
    let json: Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let default_width = json.get("width").and_then(Value::as_u64).unwrap_or(24);
    let default_height = json.get("height").and_then(Value::as_u64).unwrap_or(24);
    let icons = json.get("icons")?.as_object()?;
    let mut svgs = Vec::with_capacity(size);
    for (index, (name, icon)) in icons.iter().take(size).enumerate() {
        let width = icon
            .get("width")
            .and_then(Value::as_u64)
            .unwrap_or(default_width);
        let height = icon
            .get("height")
            .and_then(Value::as_u64)
            .unwrap_or(default_height);
        let body = icon.get("body")?.as_str()?;
        let view_width = width + (index as u64 % 5) * (width / 2).max(1);
        let view_height = height + ((index as u64 * 3) % 7) * (height / 3).max(1);
        svgs.push((
            name.replace(['/', ':'], "-"),
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {view_width} {view_height}\">{body}</svg>"
            ),
        ));
    }
    (svgs.len() == size).then_some(svgs)
}

fn fixtures(size: usize) -> FixtureSet {
    let dir = temp_dir(&format!("{size}"));
    let mut paths = Vec::with_capacity(size);
    let mut sources = Vec::with_capacity(size);
    let iconify = iconify_svgs(size);
    for index in 0..size {
        let (name, contents) = iconify
            .as_ref()
            .and_then(|icons| icons.get(index).cloned())
            .unwrap_or_else(|| (format!("icon-{index:04}"), svg(&path_data(index))));
        let path = dir.join(format!("{name}.svg"));
        std::fs::write(&path, &contents).unwrap();
        let path = path.to_string_lossy().into_owned();
        paths.push(path.clone());
        sources.push(BenchSvgSource {
            path,
            glyph_name: name,
            contents,
        });
    }
    FixtureSet {
        dir,
        paths,
        sources,
    }
}

fn format_options(woff2_quality: u8, optimize_output: bool) -> FormatOptions {
    FormatOptions {
        svg: Some(SvgFormatOptions {
            optimize_output: Some(optimize_output),
            ..Default::default()
        }),
        ttf: Some(TtfFormatOptions {
            copyright: None,
            description: None,
            ts: Some(TEST_TTF_TIMESTAMP),
            url: None,
            version: None,
        }),
        woff2: Some(Woff2FormatOptions {
            compression_quality: Some(woff2_quality),
        }),
        ..Default::default()
    }
}

fn options(
    paths: Vec<String>,
    types: Vec<FontType>,
    woff2_quality: u8,
    optimize_output: bool,
) -> GenerateWebfontsOptions {
    GenerateWebfontsOptions {
        css: Some(false),
        dest: BENCH_DEST.to_owned(),
        files: paths,
        font_name: Some("bench".to_owned()),
        format_options: Some(format_options(woff2_quality, optimize_output)),
        html: Some(false),
        ligature: Some(false),
        types: Some(types),
        write_files: Some(false),
        ..Default::default()
    }
}

fn bench_pipeline_slices(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");
    group.sample_size(10);
    for size in SIZES {
        let fixture = fixtures(size);
        group.bench_function(format!("svg/{size}"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(
                    options(fixture.paths.clone(), vec![FontType::Svg], 10, false),
                    None,
                )
                .unwrap()
            })
        });
        group.bench_function(format!("ttf/{size}"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(
                    options(fixture.paths.clone(), vec![FontType::Ttf], 10, false),
                    None,
                )
                .unwrap()
            })
        });
        group.bench_function(format!("all_formats/{size}"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(
                    options(
                        fixture.paths.clone(),
                        vec![
                            FontType::Svg,
                            FontType::Ttf,
                            FontType::Eot,
                            FontType::Woff,
                            FontType::Woff2,
                        ],
                        10,
                        false,
                    ),
                    None,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_pipeline_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_stages");
    group.sample_size(10);
    for size in SIZES {
        let fixture = fixtures(size);
        group.bench_function(format!("parse_only/{size}"), |b| {
            b.iter(|| {
                parse_svg_only(
                    options(fixture.paths.clone(), vec![FontType::Svg], 10, false),
                    &fixture.sources,
                )
                .unwrap()
            })
        });

        let parsed = parse_svg_only(
            options(fixture.paths.clone(), vec![FontType::Svg], 10, false),
            &fixture.sources,
        )
        .unwrap();
        group.bench_function(format!("finalize_only/{size}"), |b| {
            b.iter(|| {
                finalize_svg_only(
                    options(fixture.paths.clone(), vec![FontType::Svg], 10, false),
                    &fixture.sources,
                    parsed.clone(),
                )
                .unwrap()
            })
        });

        let prepared = finalize_svg_only(
            options(fixture.paths.clone(), vec![FontType::Svg], 10, false),
            &fixture.sources,
            parsed,
        )
        .unwrap();
        group.bench_function(format!("ttf_output_only/{size}"), |b| {
            b.iter(|| {
                build_outputs_only(
                    options(fixture.paths.clone(), vec![FontType::Ttf], 10, false),
                    &fixture.sources,
                    &prepared,
                )
                .unwrap()
            })
        });
        group.bench_function(format!("woff_output_only/{size}"), |b| {
            b.iter(|| {
                build_outputs_only(
                    options(fixture.paths.clone(), vec![FontType::Woff], 10, false),
                    &fixture.sources,
                    &prepared,
                )
                .unwrap()
            })
        });
        group.bench_function(format!("woff2_output_only/{size}"), |b| {
            b.iter(|| {
                build_outputs_only(
                    options(fixture.paths.clone(), vec![FontType::Woff2], 10, false),
                    &fixture.sources,
                    &prepared,
                )
                .unwrap()
            })
        });
        group.bench_function(format!("eot_output_only/{size}"), |b| {
            b.iter(|| {
                build_outputs_only(
                    options(fixture.paths.clone(), vec![FontType::Eot], 10, false),
                    &fixture.sources,
                    &prepared,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_woff2_quality(c: &mut Criterion) {
    let mut group = c.benchmark_group("woff2_quality");
    group.sample_size(10);
    let fixture = fixtures(300);
    for quality in [9, 10, 11] {
        group.bench_function(format!("quality_{quality}"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(
                    options(fixture.paths.clone(), vec![FontType::Woff2], quality, false),
                    None,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_optimize_output(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimize_output");
    group.sample_size(10);
    let fixture = fixtures(300);
    for optimize in [false, true] {
        group.bench_function(format!("optimize_{optimize}"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(
                    options(fixture.paths.clone(), vec![FontType::Svg], 10, optimize),
                    None,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = bench_pipeline_slices, bench_pipeline_stages, bench_woff2_quality, bench_optimize_output
}
criterion_main!(benches);
