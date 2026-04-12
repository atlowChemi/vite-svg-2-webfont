import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { bench, describe, expect, type BenchOptions } from 'vite-plus/test';
import { generateWebfonts, type GenerateWebfontsInputOptions } from '@atlowchemi/webfont-generator';

const require = createRequire(import.meta.url);
const upstreamCallback = require('@vusion/webfonts-generator') as (options: GenerateWebfontsInputOptions, done: (error: unknown, result?: unknown) => void) => void;
const upstreamDirect = promisify(upstreamCallback) as unknown as (options: GenerateWebfontsInputOptions) => Promise<unknown>;

// --- Fixture setup ---
// Generate 600 SVG icons for stress tests
const bulkFixtureDir = await mkdtemp(join(tmpdir(), '__bench-bulk-svgs-'));
const bulkFiles: string[] = [];
const fileWritePromises: Promise<void>[] = [];
for (let i = 0; i < 600; i++) {
    const cx = 20 + (i % 60);
    const cy = 20 + Math.floor(i / 60) * 5;
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="${cx}" cy="${cy}" r="10"/></svg>`;
    const path = join(bulkFixtureDir, `icon-${String(i).padStart(3, '0')}.svg`);
    fileWritePromises.push(writeFile(path, svg));
    bulkFiles.push(path);
}
await Promise.all(fileWritePromises);

process.on('exit', () => {
    rm(bulkFixtureDir, { force: true, recursive: true })
        .then(() => console.log(`Cleaned up ${bulkFixtureDir}`))
        .catch(() => {});
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
    const benchOpts: BenchOptions = numGlyphs >= 100 ? { time: 2000 } : {};

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
