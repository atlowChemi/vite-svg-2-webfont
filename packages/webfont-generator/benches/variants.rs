//! Paired one-shot comparison: one three-variant family versus three ordinary fonts.
//! SVG fixture creation, option cloning, and byte/determinism checks are outside timing.

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use serde_json::json;
use webfont_generator::{
    FontType, FontVariant, FormatOptions, GenerateWebfontsOptions, GenerateWebfontsResult,
    MissingGlyphBehavior, MissingGlyphOptions, TtfFormatOptions, generate_sync,
};
use write_fonts::read::{FontRef, TableProvider};

mod support;

const SIZES: [usize; 4] = [5, 100, 300, 600];
const WEIGHTS: [u16; 3] = [300, 400, 700];

struct Fixtures {
    root: PathBuf,
    files: Vec<Vec<String>>,
}

impl Drop for Fixtures {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

impl Fixtures {
    fn new(
        sources: &[(String, String)],
        count: usize,
        shared_percent: usize,
        designs: usize,
    ) -> Self {
        let root = std::env::temp_dir().join(format!(
            "webfont-variants-bench-{}-{count}-{shared_percent}-{designs}",
            std::process::id()
        ));
        let shared = count * shared_percent / 100;
        let files = (0..designs).map(|variant| {
            let directory = root.join(variant.to_string());
            std::fs::create_dir_all(&directory).unwrap();
            sources[..count].iter().enumerate().map(|(index, (_, source))| {
                // Controlled synthetic designs of real Iconify artwork. Shared cells
                // have exactly the same SVG bytes and logical filename at different paths.
                let design = if index < shared { 0 } else { variant };
                let transform = match design {
                    0 => "matrix(1 0 0 1 0 0)".to_owned(),
                    1 => "matrix(.9 0 .03 .9 .5 .5)".to_owned(),
                    2 => "matrix(.8 .02 0 .85 1 1)".to_owned(),
                    _ => {
                        let scale = 1.0 / (1.0 + design as f64 * 0.15);
                        format!("matrix({scale} 0 0 {scale} 1 1)")
                    }
                };
                let source = source.replacen("<svg ", "<svg width=\"24\" height=\"24\" ", 1);
                let svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\"><g transform=\"{transform}\">{source}</g></svg>");
                let path = directory.join(format!("icon-{index:04}.svg"));
                std::fs::write(&path, svg).unwrap();
                path.to_string_lossy().into_owned()
            }).collect()
        }).collect();
        let fixtures = Self { root, files };
        // Fail before timing if distinct designs accidentally reuse the same inputs.
        for index in 0..count {
            let contents: std::collections::HashSet<_> = fixtures
                .files
                .iter()
                .map(|files| std::fs::read(&files[index]).unwrap())
                .collect();
            assert_eq!(
                contents.len(),
                if index < shared { 1 } else { designs },
                "unexpected artwork sharing for logical icon {index}"
            );
            let paths: std::collections::HashSet<_> =
                fixtures.files.iter().map(|files| &files[index]).collect();
            assert_eq!(
                paths.len(),
                designs,
                "each design must have its own input path"
            );
        }
        fixtures
    }

    fn base_options(&self, types: &[FontType]) -> GenerateWebfontsOptions {
        GenerateWebfontsOptions {
            dest: self
                .root
                .join("unused-output")
                .to_string_lossy()
                .into_owned(),
            css: Some(false),
            html: Some(false),
            write_files: Some(false),
            incremental: Some(false),
            font_name: Some("comparison".into()),
            font_height: Some(1000.0),
            ascent: Some(1000.0),
            descent: Some(0.0),
            normalize: Some(true),
            ligature: Some(true),
            types: Some(types.to_vec()),
            format_options: Some(FormatOptions {
                ttf: Some(TtfFormatOptions {
                    ts: Some(1_700_000_000),
                    copyright: None,
                    description: None,
                    url: None,
                    version: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn options(
        &self,
        types: &[FontType],
    ) -> (GenerateWebfontsOptions, Vec<GenerateWebfontsOptions>) {
        assert_eq!(self.files.len(), WEIGHTS.len());
        let base = self.base_options(types);
        let variants = GenerateWebfontsOptions {
            variants: Some(
                self.files
                    .iter()
                    .enumerate()
                    .map(|(index, files)| FontVariant {
                        name: format!("design-{index}"),
                        files: files.clone(),
                        weight: Some(WEIGHTS[index]),
                        default: Some(index == 1),
                    })
                    .collect(),
            ),
            ..base.clone()
        };
        let ordinary = self
            .files
            .iter()
            .enumerate()
            .map(|(index, files)| GenerateWebfontsOptions {
                files: files.clone(),
                font_name: Some(format!("comparison-{index}")),
                font_weight: Some(WEIGHTS[index].to_string()),
                ..base.clone()
            })
            .collect();
        (variants, ordinary)
    }
}

fn ordinary_fonts(options: Vec<GenerateWebfontsOptions>) -> Vec<GenerateWebfontsResult> {
    options
        .into_iter()
        .map(|options| generate_sync(options, None).unwrap())
        .collect()
}

fn bytes(result: &GenerateWebfontsResult) -> [Vec<u8>; 3] {
    [
        result.ttf_bytes(),
        result.woff_bytes(),
        result.woff2_bytes(),
    ]
    .map(|value| value.unwrap_or_default().to_vec())
}

fn check_metrics(ttf: &[u8], count: usize) {
    let font = FontRef::new(ttf).unwrap();
    assert_eq!(font.head().unwrap().units_per_em(), 1000);
    assert_eq!(font.hhea().unwrap().ascender().to_i16(), 1000);
    assert_eq!(font.hhea().unwrap().descender().to_i16(), 0);
    let cmap = font.cmap().unwrap();
    let hmtx = font.hmtx().unwrap();
    for index in 0..count {
        let gid = cmap.map_codepoint(0xf101 + index as u32).unwrap().to_u32() as usize;
        let metric = hmtx
            .h_metrics()
            .get(gid)
            .unwrap_or_else(|| hmtx.h_metrics().last().unwrap());
        assert_eq!(metric.advance(), 1000, "logical icon {index}");
    }
}

fn compare(c: &mut Criterion) {
    let sources =
        support::iconify_svgs(600).expect("install the Iconify benchmark dataset with vp install");
    let mut sizes = Vec::new();
    for count in SIZES {
        for shared_percent in [0, 50, 100] {
            let fixtures = Fixtures::new(&sources, count, shared_percent, 3);
            for (format, types) in [
                ("ttf", vec![FontType::Ttf]),
                ("woff2", vec![FontType::Woff2]),
                (
                    "modern",
                    vec![FontType::Ttf, FontType::Woff, FontType::Woff2],
                ),
            ] {
                let (variant_options, ordinary_options) = fixtures.options(&types);
                let variant = bytes(&generate_sync(variant_options.clone(), None).unwrap());
                let ordinary: Vec<_> = ordinary_fonts(ordinary_options.clone())
                    .iter()
                    .map(bytes)
                    .collect();
                if format == "ttf" {
                    check_metrics(&variant[0], count);
                    for font in &ordinary {
                        check_metrics(&font[0], count);
                    }
                }
                assert_eq!(
                    variant,
                    bytes(&generate_sync(variant_options.clone(), None).unwrap())
                );
                assert_eq!(
                    ordinary,
                    ordinary_fonts(ordinary_options.clone())
                        .iter()
                        .map(bytes)
                        .collect::<Vec<_>>()
                );
                let variant_bytes: Vec<_> = variant.iter().map(Vec::len).collect();
                let ordinary_bytes: Vec<usize> = (0..3)
                    .map(|format| ordinary.iter().map(|font| font[format].len()).sum())
                    .collect();
                let group_name = format!("three-variants/{count}/shared-{shared_percent}/{format}");
                sizes.push(json!({"benchmark": group_name, "logical_icons":count, "svg_sources": count * 3,
                    "shared_icons":count * shared_percent / 100, "format_order":["ttf","woff","woff2"],
                    "variant_bytes":variant_bytes, "ordinary_bytes":ordinary_bytes}));
                let mut group = c.benchmark_group(group_name);
                group.sample_size(20);
                group.warm_up_time(Duration::from_secs(1));
                group.measurement_time(Duration::from_secs(3));
                group.bench_function("family", |b| {
                    b.iter_batched(
                        || variant_options.clone(),
                        |options| black_box(generate_sync(options, None).unwrap()),
                        BatchSize::SmallInput,
                    )
                });
                group.bench_function("three-ordinary", |b| {
                    b.iter_batched(
                        || ordinary_options.clone(),
                        |options| black_box(ordinary_fonts(options)),
                        BatchSize::SmallInput,
                    )
                });
                group.finish();
            }
        }
    }
    let output =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/variant-comparison-sizes.json");
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    std::fs::write(output, serde_json::to_vec_pretty(&sizes).unwrap()).unwrap();
}

// Each non-shared icon has distinct artwork for every design. Sparse designs omit
// alternating icons; design 0 is complete.
fn workloads(c: &mut Criterion) {
    let sources = support::iconify_svgs(600).expect("install the Iconify benchmark dataset");
    for count in [100, 600] {
        for designs in [2, 3, 8] {
            let fixtures = Fixtures::new(&sources, count, 50, designs);
            for coverage in ["full", "blank", "fallback"] {
                for ligature in [false, true] {
                    for (format, types) in [
                        ("ttf", vec![FontType::Ttf]),
                        ("woff2", vec![FontType::Woff2]),
                        (
                            "modern",
                            vec![FontType::Ttf, FontType::Woff, FontType::Woff2],
                        ),
                    ] {
                        let mut options = fixtures.base_options(&types);
                        options.ligature = Some(ligature);
                        options.variants = Some(
                            (0..designs)
                                .map(|design| FontVariant {
                                    name: format!("design-{design}"),
                                    files: fixtures.files[design]
                                        .iter()
                                        .enumerate()
                                        .filter(|(index, _)| {
                                            coverage == "full"
                                                || design == 0
                                                || (index + design) % 2 == 0
                                        })
                                        .map(|(_, file)| file.clone())
                                        .collect(),
                                    weight: None,
                                    default: Some(design == 0),
                                })
                                .collect(),
                        );
                        options.missing_glyphs = Some(MissingGlyphOptions {
                            behavior: match coverage {
                                "blank" => MissingGlyphBehavior::Blank,
                                "fallback" => MissingGlyphBehavior::Fallback,
                                _ => MissingGlyphBehavior::Error,
                            },
                            variant: (coverage == "fallback").then(|| "design-0".into()),
                        });
                        let first = bytes(&generate_sync(options.clone(), None).unwrap());
                        assert_eq!(first, bytes(&generate_sync(options.clone(), None).unwrap()));
                        if format == "ttf" {
                            check_metrics(&first[0], count);
                        }
                        let mode = if ligature { "ligatures" } else { "direct" };
                        let mut group = c.benchmark_group(format!(
                            "variant-workloads/{count}/{designs}/{coverage}/{mode}/{format}"
                        ));
                        group.sample_size(20);
                        group.warm_up_time(Duration::from_secs(1));
                        group.measurement_time(Duration::from_secs(3));
                        group.bench_function("family", |b| {
                            b.iter_batched(
                                || options.clone(),
                                |options| black_box(generate_sync(options, None).unwrap()),
                                BatchSize::SmallInput,
                            )
                        });
                        group.finish();
                    }
                }
            }
        }
    }
}

criterion_group!(benches, compare, workloads);
criterion_main!(benches);
