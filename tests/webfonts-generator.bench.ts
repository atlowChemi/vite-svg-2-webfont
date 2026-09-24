import { join } from 'node:path';
import { rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { describe, expect, test, type BenchRunOptions as BenchOptions } from 'vite-plus/test';
import { generateWebfonts, type GenerateWebfontsFileOptions as GenerateWebfontsInputOptions } from '@atlowchemi/webfont-generator';

const require = createRequire(import.meta.url);
const upstreamCallback = require('@vusion/webfonts-generator') as (options: GenerateWebfontsInputOptions, done: (error: unknown, result?: unknown) => void) => void;
const upstreamDirect = promisify(upstreamCallback);

// --- Fixture setup ---
// Materialize 600 real icons from an Iconify set, instead of synthetic circles, so the
// per-icon parse/optimize cost (the work an incremental cache would save) is realistic.
// Pick the set via BENCH_ICON_SET: 'simple-icons' (default — single-path monochrome, like
// typical webfont icons) or 'logos' (heavy multi-path/multi-color, a stress upper-bound).
const ICON_SET = process.env.BENCH_ICON_SET || 'simple-icons';
const iconSet = require(`@iconify-json/${ICON_SET}/icons.json`) as {
    width?: number;
    height?: number;
    icons: Record<string, { body: string; width?: number; height?: number }>;
};
const bulkFixtureDir = await mkdtemp(join(tmpdir(), '__bench-bulk-svgs-'));
const bulkFiles: string[] = [];
const fileWritePromises: Promise<void>[] = [];
const iconNames = Object.keys(iconSet.icons).slice(0, 600);
iconNames.forEach((name, i) => {
    const icon = iconSet.icons[name]!;
    const w = icon.width ?? iconSet.width ?? 24;
    const h = icon.height ?? iconSet.height ?? 24;
    // Vary the em box per icon (deterministically) so glyph dimensions and aspect ratios differ
    // across the set instead of being uniform. The body stays in its original coordinate space —
    // the icons won't render "correctly", but that's irrelevant to the bench; the point is to give
    // the pipeline a non-uniform set so the normalize/global-metric recomputation is realistic.
    const vbW = w + (i % 5) * Math.round(w / 2); // cycles e.g. 24, 36, 48, 60, 72
    const vbH = h + ((i * 3) % 7) * Math.round(h / 3); // independent spread → mixed aspect ratios
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 ${vbW} ${vbH}">${icon.body}</svg>`;
    const path = join(bulkFixtureDir, `icon-${String(i).padStart(3, '0')}.svg`);
    fileWritePromises.push(writeFile(path, svg));
    bulkFiles.push(path);
});
await Promise.all(fileWritePromises);

process.on('exit', () => {
    rmSync(bulkFixtureDir, { force: true, recursive: true });
});

// --- Helpers ---

// Include warmup and every implementation; 600-glyph batched edits can take several minutes with a debug binding.
const BENCH_TIMEOUT = 600_000;
// Preserve v4's sampling defaults; v5's engine defaults to 64 iterations and 1 second.
const DEFAULT_BENCH_OPTIONS: BenchOptions = { iterations: 10, time: 500, warmupIterations: 5, warmupTime: 100 };

function baseOpts(files: string[], overrides: Partial<GenerateWebfontsInputOptions> = {}): GenerateWebfontsInputOptions {
    return {
        dest: bulkFixtureDir, // throwaway dest, writeFiles defaults to true but we override below
        files,
        fontName: 'bench-font',
        types: ['svg', 'ttf', 'eot', 'woff', 'woff2'],
        writeFiles: false,
        ...overrides,
    };
}

// --- Benchmarks ---

test('error — empty files', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
    const opts = baseOpts([], { dest: bulkFixtureDir });
    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(opts)).rejects.toBeDefined();
        }),
        bench('new core', async () => {
            await expect(generateWebfonts(opts)).rejects.toBeDefined();
        }),
        DEFAULT_BENCH_OPTIONS,
    );
});

test('error — missing dest', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
    const opts = baseOpts(bulkFiles, { dest: undefined as never });
    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(opts)).rejects.toBeDefined();
        }),
        bench('new core', async () => {
            await expect(generateWebfonts(opts)).rejects.toBeDefined();
        }),
        DEFAULT_BENCH_OPTIONS,
    );
});

test('with cssContext and htmlContext (css: true, html: true)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
    const opts = baseOpts(bulkFiles, {
        css: true,
        html: true,
        cssContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
        htmlContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
    });
    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(opts)).resolves.toBeDefined();
        }),
        bench('new core', async () => {
            await expect(generateWebfonts(opts)).resolves.toBeDefined();
        }),
        DEFAULT_BENCH_OPTIONS,
    );
});

test('with cssContext and htmlContext (css: false, html: false)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
    const opts = baseOpts(bulkFiles, {
        css: false,
        html: false,
        cssContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
        htmlContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
    });
    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(opts)).resolves.toBeDefined();
        }),
        bench('new core', async () => {
            await expect(generateWebfonts(opts)).resolves.toBeDefined();
        }),
        DEFAULT_BENCH_OPTIONS,
    );
});

test('with cssContext only (css: false)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
    const opts = baseOpts(bulkFiles, {
        css: false,
        html: false,
        cssContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
    });
    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(opts)).resolves.toBeDefined();
        }),
        bench('new core', async () => {
            await expect(generateWebfonts(opts)).resolves.toBeDefined();
        }),
        DEFAULT_BENCH_OPTIONS,
    );
});

test('with htmlContext only (html: false)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
    const opts = baseOpts(bulkFiles, {
        css: false,
        html: false,
        htmlContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
    });
    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(opts)).resolves.toBeDefined();
        }),
        bench('new core', async () => {
            await expect(generateWebfonts(opts)).resolves.toBeDefined();
        }),
        DEFAULT_BENCH_OPTIONS,
    );
});

test('with custom CSS template', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
    const opts = baseOpts(bulkFiles, {
        css: true,
        cssTemplate: join(fileURLToPath(new URL('./fixtures/templates/', import.meta.url)), 'customTemplate.hbs'),
    });
    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(opts)).resolves.toBeDefined();
        }),
        bench('new core', async () => {
            await expect(generateWebfonts(opts)).resolves.toBeDefined();
        }),
        DEFAULT_BENCH_OPTIONS,
    );
});

describe.each([15, 100, 300, 600])('%i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    // Longer sampling windows + warmup to keep rme low (warmup discards cold-start/GC outliers).
    // Scaled by glyph count so total runtime stays bounded: bigger fonts run fewer ops/sec, so a
    // fixed time window already yields plenty of samples at small N.
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300
            ? { time: 10_000, warmupTime: 1_000, warmupIterations: 10 }
            : numGlyphs >= 100
              ? { time: 8_000, warmupTime: 500, warmupIterations: 20 }
              : { time: 3_000, warmupTime: 300, warmupIterations: 50 }),
    };

    describe.each([true, false])('optimize SVG: %s', optimizeOutput => {
        test('all formats', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            const opts = baseOpts(files, { optimizeOutput });
            await bench.compare(
                bench('upstream', async () => {
                    await expect(upstreamDirect(opts)).resolves.toBeDefined();
                }),
                bench('new core', async () => {
                    await expect(generateWebfonts(opts)).resolves.toBeDefined();
                }),
                benchOpts,
            );
        });

        test('WOFF2 only', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            const opts = baseOpts(files, { types: ['woff2'], optimizeOutput });
            await bench.compare(
                bench('upstream', async () => {
                    await expect(upstreamDirect(opts)).resolves.toBeDefined();
                }),
                bench('new core', async () => {
                    await expect(generateWebfonts(opts)).resolves.toBeDefined();
                }),
                benchOpts,
            );
        });

        test('WOFF + WOFF2', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            const opts = baseOpts(files, { types: ['woff', 'woff2'], optimizeOutput });
            await bench.compare(
                bench('upstream', async () => {
                    await expect(upstreamDirect(opts)).resolves.toBeDefined();
                }),
                bench('new core', async () => {
                    await expect(generateWebfonts(opts)).resolves.toBeDefined();
                }),
                benchOpts,
            );
        });

        test('SVG only', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            const opts = baseOpts(files, { types: ['svg'], optimizeOutput });
            await bench.compare(
                bench('upstream', async () => {
                    await expect(upstreamDirect(opts)).resolves.toBeDefined();
                }),
                bench('new core', async () => {
                    await expect(generateWebfonts(opts)).resolves.toBeDefined();
                }),
                benchOpts,
            );
        });

        test('SVG + TTF', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            const opts = baseOpts(files, { types: ['svg', 'ttf'], optimizeOutput });
            await bench.compare(
                bench('upstream', async () => {
                    await expect(upstreamDirect(opts)).resolves.toBeDefined();
                }),
                bench('new core', async () => {
                    await expect(generateWebfonts(opts)).resolves.toBeDefined();
                }),
                benchOpts,
            );
        });

        test('all except WOFF2', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            const opts = baseOpts(files, { types: ['svg', 'ttf', 'eot', 'woff'], optimizeOutput });
            await bench.compare(
                bench('upstream', async () => {
                    await expect(upstreamDirect(opts)).resolves.toBeDefined();
                }),
                bench('new core', async () => {
                    await expect(generateWebfonts(opts)).resolves.toBeDefined();
                }),
                benchOpts,
            );
        });
    });

    test('with rename callback', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
        const opts = baseOpts(files, {
            types: ['svg'],
            rename: (name: string) => `prefixed-${name}`,
        });
        await bench.compare(
            bench('upstream', async () => {
                await expect(upstreamDirect(opts)).resolves.toBeDefined();
            }),
            bench('new core', async () => {
                await expect(generateWebfonts(opts)).resolves.toBeDefined();
            }),
            benchOpts,
        );
    });

    describe.each([true, false])('css: %s', css => {
        test.for([true, false])('html: %s', { timeout: BENCH_TIMEOUT }, async (html, { bench }) => {
            const upstreamDest = join(bulkFixtureDir, `write-${numGlyphs}-css${css}-html${html}-upstream`);
            const newCoreDest = join(bulkFixtureDir, `write-${numGlyphs}-css${css}-html${html}-newcore`);
            await bench.compare(
                bench('upstream', async () => {
                    await expect(upstreamDirect(baseOpts(files, { css, html, types: ['svg'], dest: `${upstreamDest}/`, writeFiles: true }))).resolves.toBeDefined();
                }),
                bench('new core', async () => {
                    await expect(generateWebfonts(baseOpts(files, { css, html, types: ['svg'], dest: `${newCoreDest}/`, writeFiles: true }))).resolves.toBeDefined();
                }),
                benchOpts,
            );
        });
    });
});

// --- Template rendering benchmarks (generateCss / generateHtml) ---

const customCssTemplate = join(fileURLToPath(new URL('./fixtures/templates/', import.meta.url)), 'customTemplate.hbs');
const customHtmlTemplate = join(fileURLToPath(new URL('./fixtures/templates/', import.meta.url)), 'customTemplate.hbs');
const dependencyAwareCssTemplate = join(bulkFixtureDir, 'dependency-aware-css.hbs');
const dependencyAwareHtmlTemplate = join(bulkFixtureDir, 'dependency-aware-html.hbs');
writeFileSync(dependencyAwareCssTemplate, '{{fontName}}\n');
writeFileSync(dependencyAwareHtmlTemplate, '<h1>{{fontName}}</h1>\n');
const contextMutator = (ctx: Record<string, unknown>) => {
    ctx.custom = 'bench-value';
};

type TemplateResult = { generateCss: (urls?: Record<string, string>) => string; generateHtml: (urls?: Record<string, string>) => string };

// Pre-generate fonts for template benchmarks (top-level await, runs once before any bench)
const templateFixtures = await (async () => {
    const configs = [5, 300].flatMap(numGlyphs => {
        const files = bulkFiles.slice(0, numGlyphs);
        return [
            { key: `${numGlyphs}-default`, opts: baseOpts(files, { css: true, html: true, writeFiles: false }) },
            { key: `${numGlyphs}-custom`, opts: baseOpts(files, { css: true, html: true, cssTemplate: customCssTemplate, htmlTemplate: customHtmlTemplate, writeFiles: false }) },
            {
                key: `${numGlyphs}-context-no-write`,
                opts: baseOpts(files, { css: false, html: false, writeFiles: false, cssContext: contextMutator, htmlContext: contextMutator }),
            },
        ];
    });
    const results = new Map<string, { upstream: TemplateResult; newCore: TemplateResult }>();
    await Promise.all(
        configs.map(async ({ key, opts }) => {
            const [upstream, newCore] = await Promise.all([upstreamDirect(opts) as Promise<TemplateResult>, generateWebfonts(opts) as Promise<TemplateResult>]);
            results.set(key, { upstream, newCore });
        }),
    );
    return results;
})();

const templateUrls = { svg: '/assets/font.svg', ttf: '/assets/font.ttf', woff: '/assets/font.woff', woff2: '/assets/font.woff2', eot: '/assets/font.eot' };

describe.each([5, 300])('generateCss / generateHtml — %i glyphs', numGlyphs => {
    const benchOpts: BenchOptions = { ...DEFAULT_BENCH_OPTIONS, ...(numGlyphs >= 300 ? { time: 2000 } : {}) };

    describe('default templates', () => {
        const { upstream, newCore } = templateFixtures.get(`${numGlyphs}-default`)!;

        test('generateCss()', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateCss()).toBeDefined()),
                bench('new core', () => expect(newCore.generateCss()).toBeDefined()),
                benchOpts,
            );
        });

        test('generateCss(urls)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateCss(templateUrls)).toBeDefined()),
                bench('new core', () => expect(newCore.generateCss(templateUrls)).toBeDefined()),
                benchOpts,
            );
        });

        test('generateHtml()', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateHtml()).toBeDefined()),
                bench('new core', () => expect(newCore.generateHtml()).toBeDefined()),
                benchOpts,
            );
        });

        test('generateHtml(urls)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateHtml(templateUrls)).toBeDefined()),
                bench('new core', () => expect(newCore.generateHtml(templateUrls)).toBeDefined()),
                benchOpts,
            );
        });
    });

    describe('custom templates', () => {
        const { upstream, newCore } = templateFixtures.get(`${numGlyphs}-custom`)!;

        test('generateCss()', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateCss()).toBeDefined()),
                bench('new core', () => expect(newCore.generateCss()).toBeDefined()),
                benchOpts,
            );
        });

        test('generateHtml()', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateHtml()).toBeDefined()),
                bench('new core', () => expect(newCore.generateHtml()).toBeDefined()),
                benchOpts,
            );
        });
    });

    describe('with context callbacks (css: false, html: false)', () => {
        const { upstream, newCore } = templateFixtures.get(`${numGlyphs}-context-no-write`)!;

        test('generateCss()', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateCss()).toBeDefined()),
                bench('new core', () => expect(newCore.generateCss()).toBeDefined()),
                benchOpts,
            );
        });

        test('generateCss(urls)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateCss(templateUrls)).toBeDefined()),
                bench('new core', () => expect(newCore.generateCss(templateUrls)).toBeDefined()),
                benchOpts,
            );
        });

        test('generateHtml()', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateHtml()).toBeDefined()),
                bench('new core', () => expect(newCore.generateHtml()).toBeDefined()),
                benchOpts,
            );
        });

        test('generateHtml(urls)', { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            await bench.compare(
                bench('upstream', () => expect(upstream.generateHtml(templateUrls)).toBeDefined()),
                bench('new core', () => expect(newCore.generateHtml(templateUrls)).toBeDefined()),
                benchOpts,
            );
        });
    });
});

// Incremental rebuild: full regen vs reusing unchanged glyphs.
const DEV_FORMAT = { formatOptions: { woff2: { compressionQuality: 10 } } };
const RENDER_REUSE_FORMAT = { optimizeOutput: false, types: ['svg'] } satisfies Partial<GenerateWebfontsInputOptions>;
const EDIT_SVG_A = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2h20v20H2z"/></svg>';
const EDIT_SVG_B = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2h20L12 22z"/></svg>';
const EDIT_SVG_C = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2h30L12 22z"/></svg>';

async function makeNEditableFiles(n: number, numGlyphs: number, label: string): Promise<string[]> {
    const filePathGenerator = (index: number) => join(bulkFixtureDir, `${label}-${numGlyphs}-edit-${String.fromCharCode(97 + index)}.svg`);
    const files = Array.from({ length: n }, (_, i) => filePathGenerator(i));
    await Promise.all(files.map(file => writeFile(file, EDIT_SVG_A)));
    return [...files, ...bulkFiles.slice(n, numGlyphs)];
}

const incrementalResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const result = await generateWebfonts(baseOpts(bulkFiles.slice(0, numGlyphs), { incremental: true, ...DEV_FORMAT }));
        incrementalResults.set(numGlyphs, result);
    }),
);

test.for([100, 300, 600])('repeated output getters — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const result = incrementalResults.get(numGlyphs)!;
    const benchOpts: BenchOptions = { ...DEFAULT_BENCH_OPTIONS, time: 1_000, warmupTime: 100, warmupIterations: 20 };

    await bench.compare(
        bench('svg', () => expect(result.svg).toBeDefined()),
        bench('ttf', () => expect(result.ttf).toBeDefined()),
        bench('eot', () => expect(result.eot).toBeDefined()),
        bench('woff', () => expect(result.woff).toBeDefined()),
        bench('woff2', () => expect(result.woff2).toBeDefined()),
        benchOpts,
    );
});

test.for([100, 300, 600])('changed event with unchanged contents — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = bulkFiles.slice(0, numGlyphs);
    const opts = baseOpts(files, DEV_FORMAT);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const result = incrementalResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    await bench.compare(
        bench('upstream — full regen', async () => {
            await expect(upstreamDirect(opts)).resolves.toBeDefined();
        }),
        bench('new core — full regen (legacy)', async () => {
            await expect(generateWebfonts(opts)).resolves.toBeDefined();
        }),
        bench('new core — incremental regenerate', () => expect(result.regenerate({ files }, change)).toBeUndefined()),
        benchOpts,
    );
});

const contentEditFiles = new Map<number, string[]>();
const contentEditResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
const asyncContentEditFiles = new Map<number, string[]>();
const asyncContentEditResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
const rediffContentEditFiles = new Map<number, string[]>();
const rediffContentEditResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const files = await makeNEditableFiles(1, numGlyphs, 'regen-content');
        const result = await generateWebfonts(baseOpts(files, { incremental: true, ...DEV_FORMAT }));
        const asyncFiles = await makeNEditableFiles(1, numGlyphs, 'regen-content-async');
        const asyncResult = await generateWebfonts(baseOpts(asyncFiles, { incremental: true, ...DEV_FORMAT }));
        const rediffFiles = await makeNEditableFiles(1, numGlyphs, 'regen-content-rediff');
        const rediffResult = await generateWebfonts(baseOpts(rediffFiles, { incremental: true, ...DEV_FORMAT }));
        contentEditFiles.set(numGlyphs, files);
        contentEditResults.set(numGlyphs, result);
        asyncContentEditFiles.set(numGlyphs, asyncFiles);
        asyncContentEditResults.set(numGlyphs, asyncResult);
        rediffContentEditFiles.set(numGlyphs, rediffFiles);
        rediffContentEditResults.set(numGlyphs, rediffResult);
    }),
);

test.for([100, 300, 600])('rebuild after a 1-file content edit — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = contentEditFiles.get(numGlyphs)!;
    // Comparisons interleave iterations, so full rebuilds must not read another case's edits.
    const fullFiles = await makeNEditableFiles(1, numGlyphs, 'regen-content-full');
    const opts = baseOpts(fullFiles, DEV_FORMAT);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const result = contentEditResults.get(numGlyphs)!;
    const asyncFiles = asyncContentEditFiles.get(numGlyphs)!;
    let asyncResult = asyncContentEditResults.get(numGlyphs)!;
    const rediffFiles = rediffContentEditFiles.get(numGlyphs)!;
    const rediffResult = rediffContentEditResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];
    const asyncChange = [{ path: asyncFiles[0]!, changeType: 'changed' as const }];
    let toggle = false;
    let asyncToggle = false;
    let rediffToggle = false;

    await bench.compare(
        bench('upstream — full regen', async () => {
            await expect(upstreamDirect(opts)).resolves.toBeDefined();
        }),
        bench('new core — full regen (legacy)', async () => {
            await expect(generateWebfonts(opts)).resolves.toBeDefined();
        }),
        bench('new core — incremental regenerate', () => {
            toggle = !toggle;
            writeFileSync(files[0]!, toggle ? EDIT_SVG_B : EDIT_SVG_A);
            expect(result.regenerate({ files }, change)).toBeUndefined();
        }),
        bench('new core — async incremental regenerate', async () => {
            asyncToggle = !asyncToggle;
            writeFileSync(asyncFiles[0]!, asyncToggle ? EDIT_SVG_B : EDIT_SVG_A);
            asyncResult = await asyncResult.regenerateAsync({ files: asyncFiles }, asyncChange);
            expect(asyncResult).toBeDefined();
        }),
        bench('new core — incremental regenerate rediff', () => {
            rediffToggle = !rediffToggle;
            writeFileSync(rediffFiles[0]!, rediffToggle ? EDIT_SVG_B : EDIT_SVG_A);
            expect(rediffResult.regenerate({ files: rediffFiles })).toBeUndefined();
        }),
        benchOpts,
    );
});

const separateTwoEditFiles = new Map<number, string[]>();
const separateTenEditFiles = new Map<number, string[]>();
const batchedEditFiles = new Map<number, string[]>();
const separateTwoEditResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
const separateTenEditResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
const batchedEditResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const separateTwoFiles = await makeNEditableFiles(2, numGlyphs, 'regen-separate-two-content');
        const separateTenFiles = await makeNEditableFiles(10, numGlyphs, 'regen-separate-ten-content');
        const batchedFiles = await makeNEditableFiles(10, numGlyphs, 'regen-batch-content');
        const [separateTwo, separateTen, batched] = await Promise.all([
            generateWebfonts(baseOpts(separateTwoFiles, { incremental: true, ...DEV_FORMAT })),
            generateWebfonts(baseOpts(separateTenFiles, { incremental: true, ...DEV_FORMAT })),
            generateWebfonts(baseOpts(batchedFiles, { incremental: true, ...DEV_FORMAT })),
        ]);
        separateTwoEditFiles.set(numGlyphs, separateTwoFiles);
        separateTenEditFiles.set(numGlyphs, separateTenFiles);
        batchedEditFiles.set(numGlyphs, batchedFiles);
        separateTwoEditResults.set(numGlyphs, separateTwo);
        separateTenEditResults.set(numGlyphs, separateTen);
        batchedEditResults.set(numGlyphs, batched);
    }),
);

test.for([100, 300, 600])('batched vs separate content edits — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const separateTwoFiles = separateTwoEditFiles.get(numGlyphs)!;
    const separateTenFiles = separateTenEditFiles.get(numGlyphs)!;
    const batchedFiles = batchedEditFiles.get(numGlyphs)!;
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const separateTwo = separateTwoEditResults.get(numGlyphs)!;
    const separateTen = separateTenEditResults.get(numGlyphs)!;
    const batched = batchedEditResults.get(numGlyphs)!;
    const separateTwoChanges = separateTwoFiles.slice(0, 2).map(path => ({ path, changeType: 'changed' as const }));
    const separateTenChanges = separateTenFiles.slice(0, 10).map(path => ({ path, changeType: 'changed' as const }));
    const batchedChanges = batchedFiles.slice(0, 10).map(path => ({ path, changeType: 'changed' as const }));

    let twoToggle = false;
    let tenToggle = false;
    let batchedToggle = false;

    await bench.compare(
        bench('new core — two separate incremental regenerates', () => {
            twoToggle = !twoToggle;
            separateTwoChanges.forEach(change => writeFileSync(change.path, twoToggle ? EDIT_SVG_B : EDIT_SVG_C));
            expect(separateTwoChanges.map(change => separateTwo.regenerate({ files: separateTwoFiles }, [change]))).toEqual(Array.from({ length: separateTwoChanges.length }));
        }),
        bench('new core — ten separate incremental regenerates', () => {
            tenToggle = !tenToggle;
            separateTenChanges.forEach(change => writeFileSync(change.path, tenToggle ? EDIT_SVG_B : EDIT_SVG_C));
            expect(separateTenChanges.map(change => separateTen.regenerate({ files: separateTenFiles }, [change]))).toEqual(Array.from({ length: separateTenChanges.length }));
        }),
        bench('new core — one batched incremental regenerate', () => {
            batchedToggle = !batchedToggle;
            batchedChanges.forEach(change => writeFileSync(change.path, batchedToggle ? EDIT_SVG_B : EDIT_SVG_C));
            expect(batched.regenerate({ files: batchedFiles }, batchedChanges)).toBeUndefined();
        }),
        benchOpts,
    );
});

// Rebuild plus CSS render; provided URLs make content-only edits cacheable.
const RENDER_URLS = { svg: '/f.svg', ttf: '/f.ttf', eot: '/f.eot', woff: '/f.woff', woff2: '/f.woff2' };
const ADD_POSITIONS = ['start', 'middle', 'end'] as const;
const extraSvgs = new Map<(typeof ADD_POSITIONS)[number], string>();
await Promise.all(
    ADD_POSITIONS.map(async position => {
        const path = join(bulkFixtureDir, `icon-extra-${position}.svg`);
        await writeFile(path, '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2h20v20H2z"/></svg>');
        extraSvgs.set(position, path);
    }),
);

function insertAtPosition(files: string[], extra: string, position: (typeof ADD_POSITIONS)[number]): string[] {
    if (position === 'start') {
        return [extra, ...files];
    }
    if (position === 'end') {
        return [...files, extra];
    }
    const index = Math.floor(files.length / 2);
    return [...files.slice(0, index), extra, ...files.slice(index)];
}

test.for([100, 300, 600])('changed event + CSS with unchanged contents — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = bulkFiles.slice(0, numGlyphs);
    const opts = baseOpts(files, DEV_FORMAT);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const result = incrementalResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    await bench.compare(
        bench('new core — full regen + render CSS', async () => {
            await expect(generateWebfonts(opts).then(r => r.generateCss(RENDER_URLS))).resolves.toBeDefined();
        }),
        bench('new core — incremental regenerate + reuse CSS', () => {
            result.regenerate({ files }, change);
            expect(result.generateCss(RENDER_URLS)).toBeDefined();
        }),
        benchOpts,
    );
});

const contentEditCssFiles = new Map<number, string[]>();
const contentEditCssResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const files = await makeNEditableFiles(1, numGlyphs, 'regen-content-css');
        const result = await generateWebfonts(baseOpts(files, { incremental: true, ...DEV_FORMAT }));
        contentEditCssFiles.set(numGlyphs, files);
        contentEditCssResults.set(numGlyphs, result);
    }),
);

test.for([100, 300, 600])('rebuild + CSS after a 1-file content edit — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = contentEditCssFiles.get(numGlyphs)!;
    const fullFiles = await makeNEditableFiles(1, numGlyphs, 'regen-content-css-full');
    const opts = baseOpts(fullFiles, DEV_FORMAT);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const result = contentEditCssResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];
    let toggle = false;

    await bench.compare(
        bench('new core — full regen + render CSS', async () => {
            await expect(generateWebfonts(opts).then(r => r.generateCss(RENDER_URLS))).resolves.toBeDefined();
        }),
        bench('new core — incremental regenerate + reuse CSS', () => {
            toggle = !toggle;
            writeFileSync(files[0]!, toggle ? EDIT_SVG_B : EDIT_SVG_A);
            result.regenerate({ files }, change);
            expect(result.generateCss(RENDER_URLS)).toBeDefined();
        }),
        benchOpts,
    );
});

// writeFiles path: regenerate should update disk outputs when enabled.
const writeResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const dest = join(bulkFixtureDir, `regen-write-${numGlyphs}`);
        const result = await generateWebfonts(baseOpts(bulkFiles.slice(0, numGlyphs), { incremental: true, writeFiles: true, dest, ...DEV_FORMAT }));
        writeResults.set(numGlyphs, result);
    }),
);

test.for([100, 300, 600])('rebuild + writeFiles after a 1-file change — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const result = writeResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    await bench.compare(
        bench('new core — full regen + writeFiles', async () => {
            await expect(generateWebfonts(baseOpts(files, { writeFiles: true, dest: join(bulkFixtureDir, `full-write-${numGlyphs}`), ...DEV_FORMAT }))).resolves.toBeDefined();
        }),
        bench('new core — incremental regenerate + writeFiles', () => expect(result.regenerate({ files }, change)).toBeUndefined()),
        benchOpts,
    );
});

const contentEditWriteFiles = new Map<number, string[]>();
const contentEditWriteResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const files = await makeNEditableFiles(1, numGlyphs, 'regen-content-write');
        const dest = join(bulkFixtureDir, `regen-content-write-${numGlyphs}`);
        const result = await generateWebfonts(baseOpts(files, { incremental: true, writeFiles: true, dest, ...DEV_FORMAT }));
        contentEditWriteFiles.set(numGlyphs, files);
        contentEditWriteResults.set(numGlyphs, result);
    }),
);

test.for([100, 300, 600])('rebuild + writeFiles after a 1-file content edit — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = contentEditWriteFiles.get(numGlyphs)!;
    const fullFiles = await makeNEditableFiles(1, numGlyphs, 'regen-content-write-full');
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const result = contentEditWriteResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];
    let toggle = false;

    await bench.compare(
        bench('new core — full regen + writeFiles', async () => {
            await expect(
                generateWebfonts(baseOpts(fullFiles, { writeFiles: true, dest: join(bulkFixtureDir, `full-content-write-${numGlyphs}`), ...DEV_FORMAT })),
            ).resolves.toBeDefined();
        }),
        bench('new core — incremental regenerate + writeFiles', () => {
            toggle = !toggle;
            writeFileSync(files[0]!, toggle ? EDIT_SVG_B : EDIT_SVG_A);
            expect(result.regenerate({ files }, change)).toBeUndefined();
        }),
        benchOpts,
    );
});

// No-op content change isolates write-skip overhead when rebuilt bytes are unchanged.
const writeSkipResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const dest = join(bulkFixtureDir, `regen-write-skip-${numGlyphs}`);
        const result = await generateWebfonts(
            baseOpts(bulkFiles.slice(0, numGlyphs), {
                css: true,
                html: true,
                incremental: true,
                writeFiles: true,
                dest,
                ...DEV_FORMAT,
            }),
        );
        writeSkipResults.set(numGlyphs, result);
    }),
);

test.for([100, 300, 600])('write-skip on unchanged outputs — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const result = writeSkipResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    await bench.compare(
        bench('new core — full regen + writeFiles', async () => {
            await expect(
                generateWebfonts(baseOpts(files, { css: true, html: true, writeFiles: true, dest: join(bulkFixtureDir, `full-write-skip-${numGlyphs}`), ...DEV_FORMAT })),
            ).resolves.toBeDefined();
        }),
        bench('new core — incremental regenerate + write-skip', () => expect(result.regenerate({ files }, change)).toBeUndefined()),
        benchOpts,
    );
});

// Ordered regenerate should keep adds/removes byte-identical at any insertion point.
const addRemoveResults = new Map<string, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].flatMap(numGlyphs =>
        ADD_POSITIONS.map(async position => {
            const result = await generateWebfonts(baseOpts(bulkFiles.slice(0, numGlyphs), { incremental: true, ...DEV_FORMAT }));
            addRemoveResults.set(`${numGlyphs}-${position}`, result);
        }),
    ),
);

describe.each([100, 300, 600])('ordered add/remove regenerate — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };

    test.for(ADD_POSITIONS)('add at %s', { timeout: BENCH_TIMEOUT }, async (position, { bench }) => {
        const extra = extraSvgs.get(position)!;
        const filesWithExtra = insertAtPosition(files, extra, position);
        const result = addRemoveResults.get(`${numGlyphs}-${position}`)!;
        let hasExtra = false;

        await bench.compare(
            bench('new core — full regen after add', async () => {
                await expect(generateWebfonts(baseOpts(filesWithExtra, DEV_FORMAT))).resolves.toBeDefined();
            }),
            bench('new core — full regen after remove', async () => {
                await expect(generateWebfonts(baseOpts(files, DEV_FORMAT))).resolves.toBeDefined();
            }),
            bench('new core — incremental add/remove toggle', () => {
                if (hasExtra) {
                    result.regenerate({ files }, [{ path: extra, changeType: 'removed' }]);
                } else {
                    result.regenerate({ files: filesWithExtra }, [{ path: extra, changeType: 'added', name: `icon-extra-${position}` }]);
                }
                hasExtra = !hasExtra;
                expect(result.svg).toBeDefined();
            }),
            benchOpts,
        );
    });
});

const dependencyAwareResults = new Map<string, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [15, 100, 300, 600].flatMap(numGlyphs => {
        const files = bulkFiles.slice(0, numGlyphs);
        return [
            generateWebfonts(baseOpts(files, { css: true, incremental: true, ...RENDER_REUSE_FORMAT })).then(
                result => (result.generateCss(), dependencyAwareResults.set(`${numGlyphs}-default-css`, result)),
            ),
            generateWebfonts(baseOpts(files, { css: true, cssTemplate: dependencyAwareCssTemplate, incremental: true, ...RENDER_REUSE_FORMAT })).then(
                result => (result.generateCss(), dependencyAwareResults.set(`${numGlyphs}-custom-css`, result)),
            ),
            generateWebfonts(baseOpts(files, { html: true, incremental: true, ...RENDER_REUSE_FORMAT })).then(
                result => (result.generateHtml(), dependencyAwareResults.set(`${numGlyphs}-default-html`, result)),
            ),
            generateWebfonts(baseOpts(files, { html: true, htmlTemplate: dependencyAwareHtmlTemplate, incremental: true, ...RENDER_REUSE_FORMAT })).then(
                result => (result.generateHtml(), dependencyAwareResults.set(`${numGlyphs}-custom-html`, result)),
            ),
        ];
    }),
);

describe.each([15, 100, 300, 600])('dependency-aware render reuse add/remove toggle — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const extra = extraSvgs.get('end')!;
    const filesWithExtra = [...files, extra];
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };

    const cases = [
        {
            key: `${numGlyphs}-default-css`,
            label: 'default CSS rerender',
            render: (result: Awaited<ReturnType<typeof generateWebfonts>>) => result.generateCss(),
        },
        {
            key: `${numGlyphs}-custom-css`,
            label: 'custom CSS ignored deps reused',
            render: (result: Awaited<ReturnType<typeof generateWebfonts>>) => result.generateCss(),
        },
        {
            key: `${numGlyphs}-default-html`,
            label: 'default HTML rerender',
            render: (result: Awaited<ReturnType<typeof generateWebfonts>>) => result.generateHtml(),
        },
        {
            key: `${numGlyphs}-custom-html`,
            label: 'custom HTML ignored deps reused',
            render: (result: Awaited<ReturnType<typeof generateWebfonts>>) => result.generateHtml(),
        },
    ];

    cases.forEach(({ key, label, render }) => {
        test(`new core — ${label} toggle`, { timeout: BENCH_TIMEOUT }, async ({ bench }) => {
            const result = dependencyAwareResults.get(key)!;
            let hasExtra = false;
            await bench(`new core — ${label} toggle`, () => {
                if (hasExtra) {
                    result.regenerate({ files }, [{ path: extra, changeType: 'removed' }]);
                } else {
                    result.regenerate({ files: filesWithExtra }, [{ path: extra, changeType: 'added', name: 'icon-extra-end' }]);
                }
                hasExtra = !hasExtra;
                expect(render(result)).toBeDefined();
            }).run(benchOpts);
        });
    });
});

// WOFF2 quality: isolate brotli speed at q9/q10/q11.
test.for([100, 300, 600])('woff2 quality — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };
    const woff2Opts = (quality: number) => baseOpts(files, { types: ['woff2'], formatOptions: { woff2: { compressionQuality: quality } } });

    await bench.compare(
        bench('upstream', async () => {
            await expect(upstreamDirect(baseOpts(files, { types: ['woff2'] }))).resolves.toBeDefined();
        }),
        bench('new core — q11', async () => {
            await expect(generateWebfonts(woff2Opts(11))).resolves.toBeDefined();
        }),
        bench('new core — q10', async () => {
            await expect(generateWebfonts(woff2Opts(10))).resolves.toBeDefined();
        }),
        bench('new core — q9', async () => {
            await expect(generateWebfonts(woff2Opts(9))).resolves.toBeDefined();
        }),
        benchOpts,
    );
});

// Initial-build overhead of retaining parsed glyphs for regenerate().
test.for([100, 300, 600])('incremental population — %i glyphs', { timeout: BENCH_TIMEOUT }, async (numGlyphs, { bench }) => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = {
        ...DEFAULT_BENCH_OPTIONS,
        ...(numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 }),
    };

    await bench.compare(
        bench('new core — incremental: false', async () => {
            await expect(generateWebfonts(baseOpts(files, { incremental: false }))).resolves.toBeDefined();
        }),
        bench('new core — incremental: true', async () => {
            await expect(generateWebfonts(baseOpts(files, { incremental: true }))).resolves.toBeDefined();
        }),
        benchOpts,
    );
});
