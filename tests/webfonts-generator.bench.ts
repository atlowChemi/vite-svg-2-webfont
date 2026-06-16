// oxlint-disable jest/no-standalone-expect
import { join } from 'node:path';
import { rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { bench, describe, expect, type BenchOptions } from 'vite-plus/test';
import { generateWebfonts, type GenerateWebfontsInputOptions } from '@atlowchemi/webfont-generator';

const require = createRequire(import.meta.url);
const upstreamCallback = require('@vusion/webfonts-generator') as (options: GenerateWebfontsInputOptions, done: (error: unknown, result?: unknown) => void) => void;
const upstreamDirect = promisify(upstreamCallback) as unknown as (options: GenerateWebfontsInputOptions) => Promise<unknown>;

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
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${vbW} ${vbH}">${icon.body}</svg>`;
    const path = join(bulkFixtureDir, `icon-${String(i).padStart(3, '0')}.svg`);
    fileWritePromises.push(writeFile(path, svg));
    bulkFiles.push(path);
});
await Promise.all(fileWritePromises);

process.on('exit', () => {
    rmSync(bulkFixtureDir, { force: true, recursive: true });
});

// --- Helpers ---

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

describe('error — empty files', () => {
    const opts = baseOpts([], { dest: bulkFixtureDir });
    bench('upstream', () => expect(upstreamDirect(opts)).rejects.toBeDefined());
    bench('new core', () => expect(generateWebfonts(opts)).rejects.toBeDefined());
});

describe('error — missing dest', () => {
    const opts = baseOpts(bulkFiles, { dest: undefined as never });
    bench('upstream', () => expect(upstreamDirect(opts)).rejects.toBeDefined());
    bench('new core', () => expect(generateWebfonts(opts)).rejects.toBeDefined());
});

describe('with cssContext and htmlContext (css: true, html: true)', () => {
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
    bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined());
    bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined());
});

describe('with cssContext and htmlContext (css: false, html: false)', () => {
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
    bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined());
    bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined());
});

describe('with cssContext only (css: false)', () => {
    const opts = baseOpts(bulkFiles, {
        css: false,
        html: false,
        cssContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
    });
    bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined());
    bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined());
});

describe('with htmlContext only (html: false)', () => {
    const opts = baseOpts(bulkFiles, {
        css: false,
        html: false,
        htmlContext: (ctx: Record<string, unknown>) => {
            ctx.custom = 'value';
        },
    });
    bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined());
    bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined());
});

describe('with custom CSS template', () => {
    const opts = baseOpts(bulkFiles, {
        css: true,
        cssTemplate: join(fileURLToPath(new URL('./fixtures/templates/', import.meta.url)), 'customTemplate.hbs'),
    });
    bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined());
    bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined());
});

describe.each([15, 100, 300, 600])('%i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    // Longer sampling windows + warmup to keep rme low (warmup discards cold-start/GC outliers).
    // Scaled by glyph count so total runtime stays bounded: bigger fonts run fewer ops/sec, so a
    // fixed time window already yields plenty of samples at small N.
    const benchOpts: BenchOptions =
        numGlyphs >= 300
            ? { time: 10_000, warmupTime: 1_000, warmupIterations: 10 }
            : numGlyphs >= 100
              ? { time: 8_000, warmupTime: 500, warmupIterations: 20 }
              : { time: 3_000, warmupTime: 300, warmupIterations: 50 };

    describe.each([true, false])('optimize SVG: %s', optimizeOutput => {
        describe('all formats', () => {
            const opts = baseOpts(files, { optimizeOutput });
            bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined(), benchOpts);
            bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined(), benchOpts);
        });

        describe('SVG only', () => {
            const opts = baseOpts(files, { types: ['svg'], optimizeOutput });
            bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined(), benchOpts);
            bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined(), benchOpts);
        });

        describe('SVG + TTF', () => {
            const opts = baseOpts(files, { types: ['svg', 'ttf'], optimizeOutput });
            bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined(), benchOpts);
            bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined(), benchOpts);
        });

        describe('all except WOFF2', () => {
            const opts = baseOpts(files, { types: ['svg', 'ttf', 'eot', 'woff'], optimizeOutput });
            bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined(), benchOpts);
            bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined(), benchOpts);
        });
    });

    describe('with rename callback', () => {
        const opts = baseOpts(files, {
            types: ['svg'],
            rename: (name: string) => `prefixed-${name}`,
        });
        bench('upstream', () => expect(upstreamDirect(opts)).resolves.toBeDefined(), benchOpts);
        bench('new core', () => expect(generateWebfonts(opts)).resolves.toBeDefined(), benchOpts);
    });

    describe.each([true, false])('css: %s', css => {
        describe.each([true, false])('html: %s', html => {
            const upstreamDest = join(bulkFixtureDir, `write-${numGlyphs}-css${css}-html${html}-upstream`);
            const newCoreDest = join(bulkFixtureDir, `write-${numGlyphs}-css${css}-html${html}-newcore`);
            bench(
                'upstream',
                () => expect(upstreamDirect(baseOpts(files, { css, html, types: ['svg'], dest: `${upstreamDest}/`, writeFiles: true }))).resolves.toBeDefined(),
                benchOpts,
            );
            bench(
                'new core',
                () => expect(generateWebfonts(baseOpts(files, { css, html, types: ['svg'], dest: `${newCoreDest}/`, writeFiles: true }))).resolves.toBeDefined(),
                benchOpts,
            );
        });
    });
});

// --- Template rendering benchmarks (generateCss / generateHtml) ---

const customCssTemplate = join(fileURLToPath(new URL('./fixtures/templates/', import.meta.url)), 'customTemplate.hbs');
const customHtmlTemplate = join(fileURLToPath(new URL('./fixtures/templates/', import.meta.url)), 'customTemplate.hbs');
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
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 2000 } : {};

    describe('default templates', () => {
        const { upstream, newCore } = templateFixtures.get(`${numGlyphs}-default`)!;

        describe('generateCss()', () => {
            bench('upstream', () => expect(upstream.generateCss()).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateCss()).toBeDefined(), benchOpts);
        });

        describe('generateCss(urls)', () => {
            bench('upstream', () => expect(upstream.generateCss(templateUrls)).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateCss(templateUrls)).toBeDefined(), benchOpts);
        });

        describe('generateHtml()', () => {
            bench('upstream', () => expect(upstream.generateHtml()).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateHtml()).toBeDefined(), benchOpts);
        });

        describe('generateHtml(urls)', () => {
            bench('upstream', () => expect(upstream.generateHtml(templateUrls)).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateHtml(templateUrls)).toBeDefined(), benchOpts);
        });
    });

    describe('custom templates', () => {
        const { upstream, newCore } = templateFixtures.get(`${numGlyphs}-custom`)!;

        describe('generateCss()', () => {
            bench('upstream', () => expect(upstream.generateCss()).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateCss()).toBeDefined(), benchOpts);
        });

        describe('generateHtml()', () => {
            bench('upstream', () => expect(upstream.generateHtml()).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateHtml()).toBeDefined(), benchOpts);
        });
    });

    describe('with context callbacks (css: false, html: false)', () => {
        const { upstream, newCore } = templateFixtures.get(`${numGlyphs}-context-no-write`)!;

        describe('generateCss()', () => {
            bench('upstream', () => expect(upstream.generateCss()).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateCss()).toBeDefined(), benchOpts);
        });

        describe('generateCss(urls)', () => {
            bench('upstream', () => expect(upstream.generateCss(templateUrls)).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateCss(templateUrls)).toBeDefined(), benchOpts);
        });

        describe('generateHtml()', () => {
            bench('upstream', () => expect(upstream.generateHtml()).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateHtml()).toBeDefined(), benchOpts);
        });

        describe('generateHtml(urls)', () => {
            bench('upstream', () => expect(upstream.generateHtml(templateUrls)).toBeDefined(), benchOpts);
            bench('new core', () => expect(newCore.generateHtml(templateUrls)).toBeDefined(), benchOpts);
        });
    });
});

// Incremental rebuild: full regen vs reusing unchanged glyphs.
const DEV_FORMAT = { formatOptions: { woff2: { compressionQuality: 10 } } };
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

describe.each([100, 300, 600])('changed event with unchanged contents — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const opts = baseOpts(files, DEV_FORMAT);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const result = incrementalResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    bench('upstream — full regen', () => expect(upstreamDirect(opts)).resolves.toBeDefined(), benchOpts);
    bench('new core — full regen (legacy)', () => expect(generateWebfonts(opts)).resolves.toBeDefined(), benchOpts);
    bench('new core — incremental regenerate', () => expect(result.regenerate(files, change)).toBeUndefined(), benchOpts);
});

const contentEditFiles = new Map<number, string[]>();
const contentEditResults = new Map<number, Awaited<ReturnType<typeof generateWebfonts>>>();
await Promise.all(
    [100, 300, 600].map(async numGlyphs => {
        const files = await makeNEditableFiles(1, numGlyphs, 'regen-content');
        const result = await generateWebfonts(baseOpts(files, { incremental: true, ...DEV_FORMAT }));
        contentEditFiles.set(numGlyphs, files);
        contentEditResults.set(numGlyphs, result);
    }),
);

describe.each([100, 300, 600])('rebuild after a 1-file content edit — %i glyphs', numGlyphs => {
    const files = contentEditFiles.get(numGlyphs)!;
    const opts = baseOpts(files, DEV_FORMAT);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const result = contentEditResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];
    let toggle = false;

    bench('upstream — full regen', () => expect(upstreamDirect(opts)).resolves.toBeDefined(), benchOpts);
    bench('new core — full regen (legacy)', () => expect(generateWebfonts(opts)).resolves.toBeDefined(), benchOpts);
    bench(
        'new core — incremental regenerate',
        () => {
            toggle = !toggle;
            writeFileSync(files[0]!, toggle ? EDIT_SVG_B : EDIT_SVG_A);
            expect(result.regenerate(files, change)).toBeUndefined();
        },
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

describe.each([100, 300, 600])('batched vs separate content edits — %i glyphs', numGlyphs => {
    const separateTwoFiles = separateTwoEditFiles.get(numGlyphs)!;
    const separateTenFiles = separateTenEditFiles.get(numGlyphs)!;
    const batchedFiles = batchedEditFiles.get(numGlyphs)!;
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const separateTwo = separateTwoEditResults.get(numGlyphs)!;
    const separateTen = separateTenEditResults.get(numGlyphs)!;
    const batched = batchedEditResults.get(numGlyphs)!;
    const separateTwoChanges = separateTwoFiles.slice(0, 2).map(path => ({ path, changeType: 'changed' as const }));
    const separateTenChanges = separateTenFiles.slice(0, 10).map(path => ({ path, changeType: 'changed' as const }));
    const batchedChanges = batchedFiles.slice(0, 10).map(path => ({ path, changeType: 'changed' as const }));

    let twoToggle = false;
    let tenToggle = false;
    let batchedToggle = false;

    bench(
        'new core — two separate incremental regenerates',
        () => {
            twoToggle = !twoToggle;
            separateTwoChanges.forEach(change => writeFileSync(change.path, twoToggle ? EDIT_SVG_B : EDIT_SVG_C));
            expect(separateTwoChanges.map(change => separateTwo.regenerate(separateTwoFiles, [change]))).toEqual(Array.from({ length: separateTwoChanges.length }));
        },
        benchOpts,
    );
    bench(
        'new core — ten separate incremental regenerates',
        () => {
            tenToggle = !tenToggle;
            separateTenChanges.forEach(change => writeFileSync(change.path, tenToggle ? EDIT_SVG_B : EDIT_SVG_C));
            expect(separateTenChanges.map(change => separateTen.regenerate(separateTenFiles, [change]))).toEqual(Array.from({ length: separateTenChanges.length }));
        },
        benchOpts,
    );
    bench(
        'new core — one batched incremental regenerate',
        () => {
            batchedToggle = !batchedToggle;
            batchedChanges.forEach(change => writeFileSync(change.path, batchedToggle ? EDIT_SVG_B : EDIT_SVG_C));
            expect(batched.regenerate(batchedFiles, batchedChanges)).toBeUndefined();
        },
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

describe.each([100, 300, 600])('changed event + CSS with unchanged contents — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const opts = baseOpts(files, DEV_FORMAT);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const result = incrementalResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    bench('new core — full regen + render CSS', () => expect(generateWebfonts(opts).then(r => r.generateCss(RENDER_URLS))).resolves.toBeDefined(), benchOpts);
    bench(
        'new core — incremental regenerate + reuse CSS',
        () => {
            result.regenerate(files, change);
            expect(result.generateCss(RENDER_URLS)).toBeDefined();
        },
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

describe.each([100, 300, 600])('rebuild + CSS after a 1-file content edit — %i glyphs', numGlyphs => {
    const files = contentEditCssFiles.get(numGlyphs)!;
    const opts = baseOpts(files, DEV_FORMAT);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const result = contentEditCssResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];
    let toggle = false;

    bench('new core — full regen + render CSS', () => expect(generateWebfonts(opts).then(r => r.generateCss(RENDER_URLS))).resolves.toBeDefined(), benchOpts);
    bench(
        'new core — incremental regenerate + reuse CSS',
        () => {
            toggle = !toggle;
            writeFileSync(files[0]!, toggle ? EDIT_SVG_B : EDIT_SVG_A);
            result.regenerate(files, change);
            expect(result.generateCss(RENDER_URLS)).toBeDefined();
        },
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

describe.each([100, 300, 600])('rebuild + writeFiles after a 1-file change — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const result = writeResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    bench(
        'new core — full regen + writeFiles',
        () => expect(generateWebfonts(baseOpts(files, { writeFiles: true, dest: join(bulkFixtureDir, `full-write-${numGlyphs}`), ...DEV_FORMAT }))).resolves.toBeDefined(),
        benchOpts,
    );
    bench('new core — incremental regenerate + writeFiles', () => expect(result.regenerate(files, change)).toBeUndefined(), benchOpts);
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

describe.each([100, 300, 600])('rebuild + writeFiles after a 1-file content edit — %i glyphs', numGlyphs => {
    const files = contentEditWriteFiles.get(numGlyphs)!;
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const result = contentEditWriteResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];
    let toggle = false;

    bench(
        'new core — full regen + writeFiles',
        () => expect(generateWebfonts(baseOpts(files, { writeFiles: true, dest: join(bulkFixtureDir, `full-content-write-${numGlyphs}`), ...DEV_FORMAT }))).resolves.toBeDefined(),
        benchOpts,
    );
    bench(
        'new core — incremental regenerate + writeFiles',
        () => {
            toggle = !toggle;
            writeFileSync(files[0]!, toggle ? EDIT_SVG_B : EDIT_SVG_A);
            expect(result.regenerate(files, change)).toBeUndefined();
        },
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

describe.each([100, 300, 600])('write-skip on unchanged outputs — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const result = writeSkipResults.get(numGlyphs)!;
    const change = [{ path: files[0]!, changeType: 'changed' as const }];

    bench(
        'new core — full regen + writeFiles',
        () =>
            expect(
                generateWebfonts(baseOpts(files, { css: true, html: true, writeFiles: true, dest: join(bulkFixtureDir, `full-write-skip-${numGlyphs}`), ...DEV_FORMAT })),
            ).resolves.toBeDefined(),
        benchOpts,
    );
    bench('new core — incremental regenerate + write-skip', () => expect(result.regenerate(files, change)).toBeUndefined(), benchOpts);
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
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };

    describe.each(ADD_POSITIONS)('add at %s', position => {
        const extra = extraSvgs.get(position)!;
        const filesWithExtra = insertAtPosition(files, extra, position);
        const result = addRemoveResults.get(`${numGlyphs}-${position}`)!;
        let hasExtra = false;

        bench('new core — full regen after add', () => expect(generateWebfonts(baseOpts(filesWithExtra, DEV_FORMAT))).resolves.toBeDefined(), benchOpts);
        bench('new core — full regen after remove', () => expect(generateWebfonts(baseOpts(files, DEV_FORMAT))).resolves.toBeDefined(), benchOpts);
        bench(
            'new core — incremental add/remove toggle',
            () => {
                if (hasExtra) {
                    result.regenerate(files, [{ path: extra, changeType: 'removed' }]);
                } else {
                    result.regenerate(filesWithExtra, [{ path: extra, changeType: 'added', name: `icon-extra-${position}` }]);
                }
                hasExtra = !hasExtra;
                expect(result.svg).toBeDefined();
            },
            benchOpts,
        );
    });
});

// WOFF2 quality: isolate brotli speed at q9/q10/q11.
describe.each([100, 300, 600])('woff2 quality — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };
    const woff2Opts = (quality: number) => baseOpts(files, { types: ['woff2'], formatOptions: { woff2: { compressionQuality: quality } } });

    bench('upstream', () => expect(upstreamDirect(baseOpts(files, { types: ['woff2'] }))).resolves.toBeDefined(), benchOpts);
    bench('new core — q11', () => expect(generateWebfonts(woff2Opts(11))).resolves.toBeDefined(), benchOpts);
    bench('new core — q10', () => expect(generateWebfonts(woff2Opts(10))).resolves.toBeDefined(), benchOpts);
    bench('new core — q9', () => expect(generateWebfonts(woff2Opts(9))).resolves.toBeDefined(), benchOpts);
});

// Initial-build overhead of retaining parsed glyphs for regenerate().
describe.each([100, 300, 600])('incremental population — %i glyphs', numGlyphs => {
    const files = bulkFiles.slice(0, numGlyphs);
    const benchOpts: BenchOptions = numGlyphs >= 300 ? { time: 8_000, warmupTime: 1_000, warmupIterations: 10 } : { time: 4_000, warmupTime: 500, warmupIterations: 20 };

    bench('new core — incremental: false', () => expect(generateWebfonts(baseOpts(files, { incremental: false }))).resolves.toBeDefined(), benchOpts);
    bench('new core — incremental: true', () => expect(generateWebfonts(baseOpts(files, { incremental: true }))).resolves.toBeDefined(), benchOpts);
});
