import { expectTypeOf, it } from 'vite-plus/test';
import type { Plugin } from 'vite-plus';
import viteSvgToWebfont, { templates, type GeneratedWebfont, type PublicApi, viteSvgToWebfont as namedExport } from './index.js';
import type { IconPluginOptions } from './index.js';

it('exports the plugin and its public API', () => {
    expectTypeOf(namedExport).toEqualTypeOf(viteSvgToWebfont);
    expectTypeOf(templates).toEqualTypeOf<{ html: string; css: string; scss: string }>();
    expectTypeOf<GeneratedWebfont>().toEqualTypeOf<{ type: 'svg' | 'ttf' | 'eot' | 'woff' | 'woff2'; href: string }>();
    expectTypeOf<PublicApi['getGeneratedWebfonts']>().returns.toEqualTypeOf<GeneratedWebfont[]>();
});

it('infers variant metadata and modern formats without weakening ordinary contracts', () => {
    viteSvgToWebfont({ context: 'icons', files: '*.svg' });
    viteSvgToWebfont({ context: 'icons', variants: [{ name: 'light', default: true, files: '*.svg' }] });
    expectTypeOf(viteSvgToWebfont<'woff2'>).toBeCallableWith({} as IconPluginOptions<'woff2'>);
    viteSvgToWebfont({
        context: 'icons',
        variants: [
            { name: 'light', default: true },
            { name: 'bold', context: 'bold', weight: 700 },
        ],
        types: ['woff2'],
        preloadFormats: 'woff2',
        missingGlyphs: { behavior: 'blank' },
        cssContext(context) {
            expectTypeOf(context.variants).toBeArray();
            expectTypeOf(context.defaultWeight).toBeNumber();
            expectTypeOf(context.variants[0]!.name).toBeString();
        },
    });
    viteSvgToWebfont({
        context: 'icons',
        cssContext(context) {
            expectTypeOf(context.variants).toBeUnknown();
        },
    });
    // @ts-expect-error Variants and top-level files are mutually exclusive.
    viteSvgToWebfont({ context: 'icons', files: '*.svg', variants: [{ name: 'light', default: true, files: '*.svg' }] });
    // @ts-expect-error Legacy formats cannot be selected for variants.
    viteSvgToWebfont({ context: 'icons', variants: [{ name: 'a', default: true }], types: ['svg'] });
    // @ts-expect-error Source modes are mutually exclusive.
    viteSvgToWebfont({ context: 'icons', variants: [{ name: 'a', default: true }], files: ['*.svg'] });
    // @ts-expect-error Preload selection cannot widen generated formats.
    viteSvgToWebfont({ context: 'icons', variants: [{ name: 'a', default: true }], types: ['ttf'], preloadFormats: 'woff2' });
});

it('preserves selected formats and plugin API types', () => {
    const plugin = viteSvgToWebfont({
        context: 'icons',
        types: ['woff', 'woff2'],
        preloadFormats: 'woff2',
        formatOptions: { woff: { metadata: '<metadata />' }, woff2: { compressionQuality: 10 } },
        cssContext(context) {
            expectTypeOf(context.fontName).toBeString();
            expectTypeOf(context.codepoints).toEqualTypeOf<Record<string, string>>();
        },
        shouldProcessHtml(context) {
            expectTypeOf(context).toExtend<{ path: string; filename: string; originalUrl?: string }>();
            return true;
        },
    });

    expectTypeOf(plugin).toExtend<Plugin<PublicApi>>();
    expectTypeOf(plugin.api).toEqualTypeOf<PublicApi | undefined>();
});

it('rejects unsupported or unselected formats', () => {
    expectTypeOf(viteSvgToWebfont).toBeFunction();

    viteSvgToWebfont({
        context: 'icons',
        types: ['woff2'],
        // @ts-expect-error only generated formats can be preloaded
        preloadFormats: 'svg',
    });

    viteSvgToWebfont({
        context: 'icons',
        // @ts-expect-error not a supported font format
        types: ['otf'],
    });
});
