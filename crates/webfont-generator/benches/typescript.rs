use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use webfont_generator::bench_support::BenchTsRenderer;
use webfont_generator::{
    FontType, GenerateWebfontsFromSourcesOptions, GenerateWebfontsFromSourcesResult, SourceChange,
    SourceMembership, SvgSource, generate_from_sources_sync,
};

// Fixtures contain only ASCII identifier names. Triple braces make output byte-identical;
// escaping of arbitrary user strings is tested separately in rendering::ts::tests.
const TEMPLATE: &str = "export const iconBaseSelector = \"{{{baseSelector}}}\" as const;\nexport const iconClassPrefix = \"{{{classPrefix}}}\" as const;\nexport const icons = [{{#each names}}{{#unless @first}},{{/unless}}\n  \"{{{this}}}\"{{/each}}\n] as const;\n\nexport type IconName = (typeof icons)[number];\n";
const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0H24V24H0Z"/></svg>"#;

fn render(result: &GenerateWebfontsFromSourcesResult, ts: bool) -> std::sync::Arc<String> {
    if ts {
        result.generate_ts().unwrap()
    } else {
        result.generate_html().unwrap()
    }
}

fn benchmarks(c: &mut Criterion) {
    for size in [16, 256, 1024] {
        let names: Vec<_> = (0..size).map(|i| format!("icon_{i}")).collect();
        let direct = BenchTsRenderer::new(names.clone());
        let context = handlebars::Context::wraps(serde_json::json!({
            "baseSelector": ".icon", "classPrefix": "icon-", "names": names,
        }))
        .unwrap();
        let mut registry = handlebars::Handlebars::new();
        registry.register_template_string("ts", TEMPLATE).unwrap();
        assert_eq!(
            direct.render(),
            registry.render_with_context("ts", &context).unwrap()
        );
        let options = GenerateWebfontsFromSourcesOptions {
            sources: names
                .iter()
                .map(|name| SvgSource {
                    id: name.clone(),
                    name: name.clone(),
                    svg: SVG.into(),
                })
                .collect(),
            types: Some(vec![FontType::Svg]),
            css: Some(false),
            incremental: Some(true),
            html_template: Some(TEMPLATE.into()),
            ..Default::default()
        };
        let result = generate_from_sources_sync(options.clone()).unwrap();
        assert_eq!(
            result.generate_ts().unwrap(),
            result.generate_html().unwrap()
        );
        let mut group = c.benchmark_group(format!("typescript/{size}"));
        group
            .sample_size(20)
            .warm_up_time(Duration::from_millis(300))
            .measurement_time(Duration::from_secs(1));
        group.bench_function("render/direct", |b| b.iter(|| black_box(direct.render())));
        group.bench_function("render/handlebars-precompiled", |b| {
            b.iter(|| {
                registry
                    .render_with_context("ts", black_box(&context))
                    .unwrap()
            })
        });
        group.bench_function("render/handlebars-cold", |b| {
            b.iter(|| {
                let mut registry = handlebars::Handlebars::new();
                registry.register_template_string("ts", TEMPLATE).unwrap();
                registry
                    .render_with_context("ts", black_box(&context))
                    .unwrap()
            })
        });
        for (label, ts) in [("direct", true), ("html-template", false)] {
            group.bench_function(format!("first-result-render/{label}"), |b| {
                b.iter_batched(
                    || generate_from_sources_sync(options.clone()).unwrap(),
                    |result| {
                        let text = black_box(render(&result, ts));
                        // Criterion drops returned values outside the timed section.
                        (result, text)
                    },
                    BatchSize::SmallInput,
                )
            });
            group.bench_function(format!("cached-result-access/{label}"), |b| {
                b.iter(|| render(black_box(&result), ts))
            });
            group.bench_function(format!("generation-and-render/{label}"), |b| {
                b.iter(|| {
                    let result = generate_from_sources_sync(options.clone()).unwrap();
                    black_box(render(&result, ts))
                })
            });
            group.bench_function(format!("rename-and-render/{label}"), |b| {
                b.iter_batched(
                    || {
                        let result = generate_from_sources_sync(options.clone()).unwrap();
                        render(&result, ts);
                        result
                    },
                    |mut result| {
                        result
                            .regenerate_sync(
                                SourceMembership::Sources(names.clone()),
                                vec![SourceChange::Changed {
                                    id: names[0].clone(),
                                    name: Some("renamed".into()),
                                    svg: SVG.into(),
                                }],
                            )
                            .unwrap();
                        let text = render(&result, ts);
                        assert!(text.contains("\"renamed\""));
                        (result, black_box(text))
                    },
                    BatchSize::SmallInput,
                )
            });
        }
        group.finish();
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
