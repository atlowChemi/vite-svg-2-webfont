import { expectTypeOf, it } from 'vite-plus/test';
import type { Plugin } from 'vite';
import viteSvgToWebfont, { templates, type GeneratedWebfont, type PublicApi, viteSvgToWebfont as namedExport } from './index.js';

it('exports the plugin and its public API', () => {
    expectTypeOf(namedExport).toEqualTypeOf(viteSvgToWebfont);
    expectTypeOf(templates).toEqualTypeOf<{ html: string; css: string; scss: string }>();
    expectTypeOf<GeneratedWebfont>().toEqualTypeOf<{ type: 'svg' | 'ttf' | 'eot' | 'woff' | 'woff2'; href: string }>();
    expectTypeOf<PublicApi['getGeneratedWebfonts']>().returns.toEqualTypeOf<GeneratedWebfont[]>();
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
