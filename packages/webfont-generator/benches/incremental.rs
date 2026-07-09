use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use serde_json::Value;
use webfont_generator::bench_support::{
    BenchGlyphCache, BenchSvgSource, clear_woff1_payload_cache, prepare_svg_full,
    prepare_svg_incremental,
};
use webfont_generator::{
    FontType, FormatOptions, GenerateWebfontsOptions, GlyphChange, TtfFormatOptions,
    Woff2FormatOptions,
};

const SIZES: [usize; 3] = [100, 300, 600];
const BENCH_DEST: &str = "/tmp/webfont-generator-bench-out";
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
        "webfont-generator-bench-{prefix}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn path_data(index: usize) -> String {
    // A deterministic but non-trivial path: enough curve/line segments to make SVG parsing and
    // path processing visible without depending on external icon packages at bench time.
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

fn changed_path_data() -> String {
    let mut d = String::from("M1 1");
    for segment in 0..24 {
        let t = segment as f64;
        let x1 = 2.0 + ((t * 2.1) % 20.0);
        let y1 = 2.0 + ((t * 1.9) % 20.0);
        let x2 = 2.0 + ((t * 3.3 + 3.0) % 20.0);
        let y2 = 2.0 + ((t * 2.7 + 4.0) % 20.0);
        let x = 2.0 + ((t * 1.5 + 6.0) % 20.0);
        let y = 2.0 + ((t * 3.5 + 8.0) % 20.0);
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

fn write_icon(dir: &Path, name: &str, path_data: &str) -> String {
    let path = dir.join(format!("{name}.svg"));
    std::fs::write(&path, svg(path_data)).unwrap();
    path.to_string_lossy().into_owned()
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
        let path = dir
            .join(format!("{name}.svg"))
            .to_string_lossy()
            .into_owned();
        std::fs::write(&path, &contents).unwrap();
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

fn options(paths: Vec<String>, incremental: bool) -> GenerateWebfontsOptions {
    options_with_types(paths, incremental, vec![FontType::Woff2])
}

fn options_with_types(
    paths: Vec<String>,
    incremental: bool,
    types: Vec<FontType>,
) -> GenerateWebfontsOptions {
    GenerateWebfontsOptions {
        css: Some(false),
        dest: BENCH_DEST.to_owned(),
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
        incremental: Some(incremental),
        ligature: Some(false),
        types: Some(types),
        write_files: Some(false),
        ..Default::default()
    }
}

fn css_html_options(paths: Vec<String>, incremental: bool) -> GenerateWebfontsOptions {
    let mut opts = options(paths, incremental);
    opts.css = Some(true);
    opts.html = Some(true);
    opts
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

fn insert_position(paths: &[String], added: String, position: &str) -> Vec<String> {
    let mut ordered = paths.to_vec();
    let index = match position {
        "start" => 0,
        "middle" => ordered.len() / 2,
        "end" => ordered.len(),
        _ => unreachable!(),
    };
    ordered.insert(index, added);
    ordered
}

fn remove_position(paths: &[String], position: &str) -> (Vec<String>, String) {
    let index = match position {
        "start" => 0,
        "middle" => paths.len() / 2,
        "end" => paths.len() - 1,
        _ => unreachable!(),
    };
    let mut ordered = paths.to_vec();
    let removed = ordered.remove(index);
    (ordered, removed)
}

fn bench_svg_prepare(c: &mut Criterion) {
    let mut group = c.benchmark_group("svg_prepare");
    for size in SIZES {
        let fixture = fixtures(size);
        group.bench_function(format!("full/{size}"), |b| {
            b.iter(|| {
                prepare_svg_full(options(fixture.paths.clone(), false), &fixture.sources).unwrap()
            })
        });

        let mut cache = BenchGlyphCache::default();
        prepare_svg_incremental(
            options(fixture.paths.clone(), true),
            &fixture.sources,
            &mut cache,
        )
        .unwrap();
        group.bench_function(format!("incremental_warm/{size}"), |b| {
            b.iter(|| {
                prepare_svg_incremental(
                    options(fixture.paths.clone(), true),
                    &fixture.sources,
                    &mut cache,
                )
                .unwrap()
            })
        });
    }
    group.finish();
}

fn bench_regenerate(c: &mut Criterion) {
    let mut group = c.benchmark_group("regenerate");
    for size in SIZES {
        let fixture = fixtures(size);
        let mut result =
            webfont_generator::generate_sync(options(fixture.paths.clone(), true), None).unwrap();
        let changed = fixture.paths[size / 2].clone();
        group.bench_function(format!("full_unchanged/{size}"), |b| {
            b.iter(|| {
                webfont_generator::generate_sync(options(fixture.paths.clone(), false), None)
                    .unwrap()
            })
        });
        group.bench_function(format!("noop_changed/{size}"), |b| {
            b.iter(|| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(changed.clone(), GlyphChange::Changed { name: None })],
                    )
                    .unwrap()
            })
        });
        let mut rediff_noop_result =
            webfont_generator::generate_sync(options(fixture.paths.clone(), true), None).unwrap();
        group.bench_function(format!("rediff_noop/{size}"), |b| {
            b.iter(|| rediff_noop_result.regenerate_all(&fixture.paths).unwrap())
        });

        group.bench_function(format!("full_content_edit/{size}"), |b| {
            b.iter_batched(
                || {
                    let fixture = fixtures(size);
                    let changed = fixture.paths[size / 2].clone();
                    std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                    fixture
                },
                |fixture| {
                    webfont_generator::generate_sync(options(fixture.paths.clone(), false), None)
                        .unwrap()
                },
                BatchSize::SmallInput,
            )
        });

        group.bench_function(format!("incremental_content_edit/{size}"), |b| {
            b.iter_batched(
                || {
                    let fixture = fixtures(size);
                    let result = webfont_generator::generate_sync(
                        options(fixture.paths.clone(), true),
                        None,
                    )
                    .unwrap();
                    let changed = fixture.paths[size / 2].clone();
                    std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                    (fixture, result, changed)
                },
                |(fixture, mut result, changed)| {
                    result
                        .regenerate(
                            &fixture.paths,
                            &[(changed, GlyphChange::Changed { name: None })],
                        )
                        .unwrap()
                },
                BatchSize::SmallInput,
            )
        });

        group.bench_function(format!("rediff_content_edit/{size}"), |b| {
            b.iter_batched(
                || {
                    let fixture = fixtures(size);
                    let result = webfont_generator::generate_sync(
                        options(fixture.paths.clone(), true),
                        None,
                    )
                    .unwrap();
                    let changed = fixture.paths[size / 2].clone();
                    std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                    (fixture, result)
                },
                |(fixture, mut result)| result.regenerate_all(&fixture.paths).unwrap(),
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn bench_regenerate_batches(c: &mut Criterion) {
    let mut group = c.benchmark_group("regenerate_batches");
    group.sample_size(30);
    let all_formats = vec![
        FontType::Svg,
        FontType::Ttf,
        FontType::Eot,
        FontType::Woff,
        FontType::Woff2,
    ];

    for size in SIZES {
        for change_count in [2, 10] {
            group.bench_function(format!("separate_{change_count}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let fixture = fixtures(size);
                        let result = webfont_generator::generate_sync(
                            options_with_types(fixture.paths.clone(), true, all_formats.clone()),
                            None,
                        )
                        .unwrap();
                        let changes = fixture
                            .paths
                            .iter()
                            .take(change_count)
                            .cloned()
                            .collect::<Vec<_>>();
                        (fixture, result, changes)
                    },
                    |(fixture, mut result, changes)| {
                        for changed in changes {
                            std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                            result
                                .regenerate(
                                    &fixture.paths,
                                    &[(changed, GlyphChange::Changed { name: None })],
                                )
                                .unwrap();
                        }
                    },
                    BatchSize::SmallInput,
                )
            });

            group.bench_function(format!("batched_{change_count}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let fixture = fixtures(size);
                        let result = webfont_generator::generate_sync(
                            options_with_types(fixture.paths.clone(), true, all_formats.clone()),
                            None,
                        )
                        .unwrap();
                        let changes = fixture
                            .paths
                            .iter()
                            .take(change_count)
                            .cloned()
                            .collect::<Vec<_>>();
                        (fixture, result, changes)
                    },
                    |(fixture, mut result, changes)| {
                        for changed in &changes {
                            std::fs::write(changed, svg(&changed_path_data())).unwrap();
                        }
                        let changes = changes
                            .into_iter()
                            .map(|path| (path, GlyphChange::Changed { name: None }))
                            .collect::<Vec<_>>();
                        result.regenerate(&fixture.paths, &changes).unwrap();
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }
    group.finish();
}

fn bench_add_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_remove");
    for size in SIZES {
        for position in ["start", "middle", "end"] {
            group.bench_function(format!("full_add_at_{position}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let mut fixture = fixtures(size);
                        let added = write_icon(
                            &fixture.dir,
                            &format!("added-{position}"),
                            &changed_path_data(),
                        );
                        fixture.paths = insert_position(&fixture.paths, added, position);
                        fixture
                    },
                    |fixture| {
                        webfont_generator::generate_sync(
                            options(fixture.paths.clone(), false),
                            None,
                        )
                        .unwrap()
                    },
                    BatchSize::SmallInput,
                )
            });

            group.bench_function(format!("incremental_add_at_{position}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let mut fixture = fixtures(size);
                        let added = write_icon(
                            &fixture.dir,
                            &format!("added-{position}"),
                            &changed_path_data(),
                        );
                        let result = webfont_generator::generate_sync(
                            options(fixture.paths.clone(), true),
                            None,
                        )
                        .unwrap();
                        fixture.paths = insert_position(&fixture.paths, added.clone(), position);
                        (fixture, result, added)
                    },
                    |(fixture, mut result, added)| {
                        result
                            .regenerate(
                                &fixture.paths,
                                &[(added, GlyphChange::Added { name: None })],
                            )
                            .unwrap()
                    },
                    BatchSize::SmallInput,
                )
            });

            group.bench_function(format!("full_remove_at_{position}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let mut fixture = fixtures(size);
                        let (ordered, _) = remove_position(&fixture.paths, position);
                        fixture.paths = ordered;
                        fixture
                    },
                    |fixture| {
                        webfont_generator::generate_sync(
                            options(fixture.paths.clone(), false),
                            None,
                        )
                        .unwrap()
                    },
                    BatchSize::SmallInput,
                )
            });

            group.bench_function(format!("incremental_remove_at_{position}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let mut fixture = fixtures(size);
                        let result = webfont_generator::generate_sync(
                            options(fixture.paths.clone(), true),
                            None,
                        )
                        .unwrap();
                        let (ordered, removed) = remove_position(&fixture.paths, position);
                        fixture.paths = ordered;
                        (fixture, result, removed)
                    },
                    |(fixture, mut result, removed)| {
                        result
                            .regenerate(&fixture.paths, &[(removed, GlyphChange::Removed)])
                            .unwrap()
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }
    group.finish();
}

fn bench_render_cache_after_regenerate(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_cache_after_regenerate");
    let size = 300;
    group.bench_function("content_edit_css_urls_reused/300", |b| {
        b.iter_batched(
            || {
                let fixture = fixtures(size);
                let result = webfont_generator::generate_sync(
                    css_html_options(fixture.paths.clone(), true),
                    None,
                )
                .unwrap();
                result.generate_css_pure(Some(urls())).unwrap();
                let changed = fixture.paths[size / 2].clone();
                std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                (fixture, result, changed)
            },
            |(fixture, mut result, changed)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(changed, GlyphChange::Changed { name: None })],
                    )
                    .unwrap();
                result.generate_css_pure(Some(urls())).unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("rename_css_rerender/300", |b| {
        b.iter_batched(
            || {
                let fixture = fixtures(size);
                let result = webfont_generator::generate_sync(
                    css_html_options(fixture.paths.clone(), true),
                    None,
                )
                .unwrap();
                result.generate_css_pure(Some(urls())).unwrap();
                let changed = fixture.paths[size / 2].clone();
                (fixture, result, changed)
            },
            |(fixture, mut result, changed)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(
                            changed,
                            GlyphChange::Changed {
                                name: Some("renamed-glyph".to_owned()),
                            },
                        )],
                    )
                    .unwrap();
                result.generate_css_pure(Some(urls())).unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("add_custom_html_ignored_deps_reused/300", |b| {
        b.iter_batched(
            || {
                let mut fixture = fixtures(size);
                let template = fixture.dir.join("html-font-name-only.hbs");
                std::fs::write(&template, "<h1>{{fontName}}</h1>\n").unwrap();
                let mut opts = css_html_options(fixture.paths.clone(), true);
                opts.html_template = Some(template.to_string_lossy().into_owned());
                let result = webfont_generator::generate_sync(opts, None).unwrap();
                result.generate_html_pure(None).unwrap();
                let added = write_icon(&fixture.dir, "added-html-reuse", &changed_path_data());
                fixture.paths.push(added.clone());
                (fixture, result, added)
            },
            |(fixture, mut result, added)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(added, GlyphChange::Added { name: None })],
                    )
                    .unwrap();
                result.generate_html_pure(None).unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("add_default_html_rerender/300", |b| {
        b.iter_batched(
            || {
                let mut fixture = fixtures(size);
                let result = webfont_generator::generate_sync(
                    css_html_options(fixture.paths.clone(), true),
                    None,
                )
                .unwrap();
                result.generate_html_pure(None).unwrap();
                let added = write_icon(&fixture.dir, "added-html-rerender", &changed_path_data());
                fixture.paths.push(added.clone());
                (fixture, result, added)
            },
            |(fixture, mut result, added)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(added, GlyphChange::Added { name: None })],
                    )
                    .unwrap();
                result.generate_html_pure(None).unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_incremental_write_content_edit(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_files_after_regenerate");
    let size = 100;
    group.bench_function("content_edit/100", |b| {
        b.iter_batched(
            || {
                let fixture = fixtures(size);
                let mut opts = css_html_options(fixture.paths.clone(), true);
                opts.dest = temp_dir("write-content-edit")
                    .to_string_lossy()
                    .into_owned();
                opts.write_files = Some(true);
                let result = webfont_generator::generate_sync(opts, None).unwrap();
                let changed = fixture.paths[size / 2].clone();
                std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                (fixture, result, changed)
            },
            |(fixture, mut result, changed)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(changed, GlyphChange::Changed { name: None })],
                    )
                    .unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_specialized_incremental_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("specialized_incremental_paths");
    let size = 300;
    group.bench_function("rename_only/300", |b| {
        b.iter_batched(
            || {
                let fixture = fixtures(size);
                let result =
                    webfont_generator::generate_sync(options(fixture.paths.clone(), true), None)
                        .unwrap();
                let changed = fixture.paths[size / 2].clone();
                (fixture, result, changed)
            },
            |(fixture, mut result, changed)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(
                            changed,
                            GlyphChange::Changed {
                                name: Some("renamed-glyph".to_owned()),
                            },
                        )],
                    )
                    .unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("rename_only_woff/300", |b| {
        b.iter_batched(
            || {
                let fixture = fixtures(size);
                let result = webfont_generator::generate_sync(
                    options_with_types(fixture.paths.clone(), true, vec![FontType::Woff]),
                    None,
                )
                .unwrap();
                let changed = fixture.paths[size / 2].clone();
                (fixture, result, changed)
            },
            |(fixture, mut result, changed)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(
                            changed,
                            GlyphChange::Changed {
                                name: Some("renamed-glyph".to_owned()),
                            },
                        )],
                    )
                    .unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("rename_only_woff_cold_cache/300", |b| {
        b.iter_batched(
            || {
                let fixture = fixtures(size);
                let mut result = webfont_generator::generate_sync(
                    options_with_types(fixture.paths.clone(), true, vec![FontType::Woff]),
                    None,
                )
                .unwrap();
                clear_woff1_payload_cache(&mut result);
                let changed = fixture.paths[size / 2].clone();
                (fixture, result, changed)
            },
            |(fixture, mut result, changed)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(
                            changed,
                            GlyphChange::Changed {
                                name: Some("renamed-glyph".to_owned()),
                            },
                        )],
                    )
                    .unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("duplicate_content_add/300", |b| {
        b.iter_batched(
            || {
                let mut fixture = fixtures(size);
                let duplicate = fixture.dir.join("duplicate-added.svg");
                std::fs::copy(&fixture.paths[0], &duplicate).unwrap();
                let duplicate = duplicate.to_string_lossy().into_owned();
                let result =
                    webfont_generator::generate_sync(options(fixture.paths.clone(), true), None)
                        .unwrap();
                fixture.paths.push(duplicate.clone());
                (fixture, result, duplicate)
            },
            |(fixture, mut result, duplicate)| {
                result
                    .regenerate(
                        &fixture.paths,
                        &[(duplicate, GlyphChange::Added { name: None })],
                    )
                    .unwrap()
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_regenerate_by_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("regenerate_by_format");
    for size in SIZES {
        for (label, types) in [
            ("svg", vec![FontType::Svg]),
            ("ttf", vec![FontType::Ttf]),
            ("woff", vec![FontType::Woff]),
            ("woff2", vec![FontType::Woff2]),
            (
                "all_formats",
                vec![
                    FontType::Svg,
                    FontType::Ttf,
                    FontType::Eot,
                    FontType::Woff,
                    FontType::Woff2,
                ],
            ),
        ] {
            group.bench_function(format!("full_content_edit/{label}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let fixture = fixtures(size);
                        let changed = fixture.paths[size / 2].clone();
                        std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                        fixture
                    },
                    |fixture| {
                        webfont_generator::generate_sync(
                            options_with_types(fixture.paths.clone(), false, types.clone()),
                            None,
                        )
                        .unwrap()
                    },
                    BatchSize::SmallInput,
                )
            });
            group.bench_function(format!("incremental_content_edit/{label}/{size}"), |b| {
                b.iter_batched(
                    || {
                        let fixture = fixtures(size);
                        let result = webfont_generator::generate_sync(
                            options_with_types(fixture.paths.clone(), true, types.clone()),
                            None,
                        )
                        .unwrap();
                        let changed = fixture.paths[size / 2].clone();
                        std::fs::write(&changed, svg(&changed_path_data())).unwrap();
                        (fixture, result, changed)
                    },
                    |(fixture, mut result, changed)| {
                        result
                            .regenerate(
                                &fixture.paths,
                                &[(changed, GlyphChange::Changed { name: None })],
                            )
                            .unwrap()
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_svg_prepare,
    bench_regenerate,
    bench_regenerate_batches,
    bench_add_remove,
    bench_regenerate_by_format,
    bench_render_cache_after_regenerate,
    bench_incremental_write_content_edit,
    bench_specialized_incremental_paths
);
criterion_main!(benches);
