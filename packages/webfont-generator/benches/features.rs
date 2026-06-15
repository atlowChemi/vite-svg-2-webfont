use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use serde_json::Value;
use webfont_generator::{
    FontType, FormatOptions, GenerateWebfontsOptions, RenameFn, SvgFormatOptions, TtfFormatOptions,
    Woff2FormatOptions,
};

const TEST_TTF_TIMESTAMP: i64 = 1_700_000_000;

struct FixtureSet {
    dir: PathBuf,
    paths: Vec<String>,
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
        "webfont-generator-feature-bench-{prefix}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn heavy_path(index: usize) -> String {
    let mut d = String::new();
    let start_x = 2.0 + (index % 5) as f64 * 0.35;
    let start_y = 2.0 + (index % 7) as f64 * 0.25;
    d.push_str(&format!("M{start_x:.2} {start_y:.2}"));
    for segment in 0..16 {
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

fn svg_path(d: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"{d}\"/></svg>"
    )
}

fn svg_for(kind: &str, index: usize) -> String {
    match kind {
        "simple" => svg_path(&format!(
            "M2 2 L{} 2 L{} {} L2 {} Z",
            12 + index % 10,
            20,
            20,
            20
        )),
        "multipath" => format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"{}\"/><path d=\"{}\"/></svg>",
            heavy_path(index),
            heavy_path(index + 17)
        ),
        "shapes" => format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><rect x=\"2\" y=\"2\" width=\"{}\" height=\"{}\"/><circle cx=\"12\" cy=\"12\" r=\"{}\"/></svg>",
            8 + index % 12,
            8 + index % 10,
            2 + index % 7
        ),
        "transformed" => format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><g transform=\"translate(2 1) rotate({})\"><path d=\"{}\"/></g></svg>",
            index % 45,
            heavy_path(index)
        ),
        "hidden" => format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"{}\"/><path display=\"none\" d=\"{}\"/></svg>",
            heavy_path(index),
            heavy_path(index + 23)
        ),
        "duplicate" => svg_path(&heavy_path(0)),
        _ => svg_path(&heavy_path(index)),
    }
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

fn fixtures(size: usize, kind: &str) -> FixtureSet {
    let dir = temp_dir(kind);
    let mut paths = Vec::with_capacity(size);
    let iconify = (kind == "iconify").then(|| iconify_svgs(size)).flatten();
    for index in 0..size {
        let (name, contents) = iconify
            .as_ref()
            .and_then(|icons| icons.get(index).cloned())
            .unwrap_or_else(|| (format!("{kind}-{index:04}"), svg_for(kind, index)));
        let path = dir.join(format!("{name}.svg"));
        std::fs::write(&path, contents).unwrap();
        paths.push(path.to_string_lossy().into_owned());
    }
    FixtureSet { dir, paths }
}

fn template_file(dir: &Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

fn base_options(paths: Vec<String>, types: Vec<FontType>) -> GenerateWebfontsOptions {
    GenerateWebfontsOptions {
        css: Some(false),
        dest: temp_dir("out").to_string_lossy().into_owned(),
        files: paths,
        font_name: Some("bench".to_owned()),
        format_options: Some(FormatOptions {
            ttf: Some(TtfFormatOptions {
                copyright: None,
                description: None,
                ts: Some(TEST_TTF_TIMESTAMP),
                url: None,
                version: None,
            }),
            woff2: Some(Woff2FormatOptions {
                compression_quality: Some(10),
            }),
            ..Default::default()
        }),
        html: Some(false),
        ligature: Some(false),
        types: Some(types),
        write_files: Some(false),
        ..Default::default()
    }
}

fn urls() -> HashMap<FontType, String> {
    HashMap::from([
        (FontType::Svg, "/font.svg".to_owned()),
        (FontType::Ttf, "/font.ttf".to_owned()),
        (FontType::Eot, "/font.eot".to_owned()),
        (FontType::Woff, "/font.woff".to_owned()),
        (FontType::Woff2, "/font.woff2".to_owned()),
    ])
}

fn bench_template_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_rendering");
    group.sample_size(10);
    let fixture = fixtures(300, "heavy");
    let mut opts = base_options(fixture.paths.clone(), vec![FontType::Woff2]);
    opts.css = Some(true);
    opts.html = Some(true);
    let result = webfont_generator::generate_sync(opts.clone(), None).unwrap();

    group.bench_function("css_first/default/300", |b| {
        b.iter_batched(
            || webfont_generator::generate_sync(opts.clone(), None).unwrap(),
            |result| result.generate_css_pure(None).unwrap(),
            BatchSize::SmallInput,
        )
    });
    result.generate_css_pure(None).unwrap();
    group.bench_function("css_cached/default/300", |b| {
        b.iter(|| result.generate_css_pure(None).unwrap())
    });
    group.bench_function("css_urls/default/300", |b| {
        b.iter(|| result.generate_css_pure(Some(urls())).unwrap())
    });
    group.bench_function("html_first/default/300", |b| {
        b.iter_batched(
            || webfont_generator::generate_sync(opts.clone(), None).unwrap(),
            |result| result.generate_html_pure(None).unwrap(),
            BatchSize::SmallInput,
        )
    });
    result.generate_html_pure(None).unwrap();
    group.bench_function("html_cached/default/300", |b| {
        b.iter(|| result.generate_html_pure(None).unwrap())
    });

    let template_dir = temp_dir("templates");
    let css_template = template_file(
        &template_dir,
        "bench.css.hbs",
        "{{fontName}} {{#each codepoints}}{{@key}}:{{this}};{{/each}} {{{src}}}",
    );
    let html_template = template_file(
        &template_dir,
        "bench.html.hbs",
        "<html>{{fontName}}{{#each names}}<i>{{this}}</i>{{/each}}<style>{{{styles}}}</style></html>",
    );
    let mut custom_opts = opts.clone();
    custom_opts.css_template = Some(css_template);
    custom_opts.html_template = Some(html_template);
    group.bench_function("css_first/custom/300", |b| {
        b.iter_batched(
            || webfont_generator::generate_sync(custom_opts.clone(), None).unwrap(),
            |result| result.generate_css_pure(None).unwrap(),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("html_first/custom/300", |b| {
        b.iter_batched(
            || webfont_generator::generate_sync(custom_opts.clone(), None).unwrap(),
            |result| result.generate_html_pure(None).unwrap(),
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_write_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_paths");
    group.sample_size(10);
    let fixture = fixtures(100, "heavy");
    group.bench_function("initial_write/100", |b| {
        b.iter_batched(
            || {
                let mut opts = base_options(fixture.paths.clone(), vec![FontType::Woff2]);
                opts.css = Some(true);
                opts.html = Some(true);
                opts.dest = temp_dir("write").to_string_lossy().into_owned();
                opts.write_files = Some(true);
                opts
            },
            |opts| webfont_generator::generate_sync(opts, None).unwrap(),
            BatchSize::SmallInput,
        )
    });
    group.bench_function("incremental_write_skip/100", |b| {
        b.iter_batched(
            || {
                let mut opts = base_options(fixture.paths.clone(), vec![FontType::Woff2]);
                opts.css = Some(true);
                opts.dest = temp_dir("write-skip").to_string_lossy().into_owned();
                opts.incremental = Some(true);
                opts.write_files = Some(true);
                let result = webfont_generator::generate_sync(opts, None).unwrap();
                (result, fixture.paths[0].clone())
            },
            |(mut result, changed)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(
                            changed,
                            webfont_generator::GlyphChange::Changed { name: None },
                        )],
                    )
                    .unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_input_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_complexity");
    group.sample_size(10);
    for kind in [
        "simple",
        "heavy",
        "multipath",
        "shapes",
        "transformed",
        "hidden",
        "iconify",
    ] {
        let fixture = fixtures(100, kind);
        group.bench_function(format!("{kind}/100"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(
                    base_options(fixture.paths.clone(), vec![FontType::Svg]),
                    None,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_geometry_options(c: &mut Criterion) {
    let mut group = c.benchmark_group("geometry_options");
    group.sample_size(10);
    let fixture = fixtures(300, "heavy");
    for name in [
        "normalize_false",
        "normalize_true",
        "fixed_width",
        "center_h",
        "center_v",
        "preserve_aspect",
        "round_2",
        "optimize_output",
    ] {
        group.bench_function(format!("{name}/300"), |b| {
            b.iter(|| {
                let mut opts = base_options(fixture.paths.clone(), vec![FontType::Svg]);
                match name {
                    "normalize_false" => opts.normalize = Some(false),
                    "normalize_true" => opts.normalize = Some(true),
                    "fixed_width" => opts.fixed_width = Some(true),
                    "center_h" => opts.center_horizontally = Some(true),
                    "center_v" => opts.center_vertically = Some(true),
                    "preserve_aspect" => opts.preserve_aspect_ratio = Some(true),
                    "round_2" => opts.round = Some(2.0),
                    "optimize_output" => {
                        opts.format_options.get_or_insert_with(Default::default).svg =
                            Some(SvgFormatOptions {
                                optimize_output: Some(true),
                                ..Default::default()
                            });
                    }
                    _ => unreachable!(),
                }
                webfont_generator::generate_sync(opts, None).unwrap()
            })
        });
    }
    group.finish();
}

fn bench_ttf_features(c: &mut Criterion) {
    let mut group = c.benchmark_group("ttf_features");
    group.sample_size(10);
    for (name, fixture, ligature, explicit) in [
        (
            "ligatures_false_unique",
            fixtures(300, "heavy"),
            false,
            false,
        ),
        ("ligatures_true_unique", fixtures(300, "heavy"), true, false),
        ("explicit_codepoints", fixtures(300, "heavy"), false, true),
        (
            "duplicate_outlines",
            fixtures(300, "duplicate"),
            false,
            false,
        ),
    ] {
        group.bench_function(format!("{name}/300"), |b| {
            b.iter(|| {
                let mut opts = base_options(fixture.paths.clone(), vec![FontType::Ttf]);
                opts.ligature = Some(ligature);
                if explicit {
                    opts.codepoints = Some(
                        fixture
                            .paths
                            .iter()
                            .enumerate()
                            .map(|(index, path)| {
                                let name = Path::new(path)
                                    .file_stem()
                                    .and_then(|stem| stem.to_str())
                                    .unwrap_or_default()
                                    .to_owned();
                                (name, 0xE000 + index as u32)
                            })
                            .collect(),
                    );
                }
                webfont_generator::generate_sync(opts, None).unwrap()
            })
        });
    }
    group.finish();
}

fn bench_error_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_paths");
    group.sample_size(10);
    let fixture = fixtures(10, "heavy");
    let missing = temp_dir("missing")
        .join("missing.svg")
        .to_string_lossy()
        .into_owned();
    let invalid_dir = temp_dir("invalid");
    let invalid = invalid_dir
        .join("invalid.svg")
        .to_string_lossy()
        .into_owned();
    std::fs::write(&invalid, "<svg><path></svg>").unwrap();
    group.bench_function("empty_files", |b| {
        b.iter(|| {
            let result = webfont_generator::generate_sync(
                base_options(Vec::new(), vec![FontType::Svg]),
                None,
            );
            assert!(result.is_err());
        })
    });
    group.bench_function("missing_file", |b| {
        b.iter(|| {
            let result = webfont_generator::generate_sync(
                base_options(vec![missing.clone()], vec![FontType::Svg]),
                None,
            );
            assert!(result.is_err());
        })
    });
    group.bench_function("invalid_svg", |b| {
        b.iter(|| {
            let result = webfont_generator::generate_sync(
                base_options(vec![invalid.clone()], vec![FontType::Svg]),
                None,
            );
            assert!(result.is_err());
        })
    });
    group.bench_function("duplicate_glyph_names", |b| {
        b.iter(|| {
            let rename: RenameFn = Box::new(|_| "duplicate".to_owned());
            let result = webfont_generator::generate_sync(
                base_options(fixture.paths.clone(), vec![FontType::Svg]),
                Some(rename),
            );
            assert!(result.is_err());
        })
    });
    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");
    group.sample_size(10);
    for size in [15, 100, 300, 600, 1000] {
        let fixture = fixtures(size, "heavy");
        group.bench_function(format!("woff2/{size}"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(
                    base_options(fixture.paths.clone(), vec![FontType::Woff2]),
                    None,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_async_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_overhead");
    group.sample_size(10);
    let fixture = fixtures(15, "heavy");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    group.bench_function("generate_sync/15", |b| {
        b.iter(|| {
            webfont_generator::generate_sync(
                base_options(fixture.paths.clone(), vec![FontType::Svg]),
                None,
            )
            .unwrap()
        })
    });
    group.bench_function("generate_async/15", |b| {
        b.iter(|| {
            runtime
                .block_on(webfont_generator::generate(
                    base_options(fixture.paths.clone(), vec![FontType::Svg]),
                    None,
                ))
                .unwrap()
        })
    });
    group.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets =
        bench_template_rendering,
        bench_write_paths,
        bench_input_complexity,
        bench_geometry_options,
        bench_ttf_features,
        bench_error_paths,
        bench_scaling,
        bench_async_overhead
}
criterion_main!(benches);
