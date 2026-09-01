import { expectTypeOf, it } from 'vite-plus/test';
import { GenerateWebfontsResult as NativeGenerateWebfontsResult, generateWebfonts as generateNativeWebfonts } from '../binding.js';
import { generateWebfonts, templates, type CssContext, type FormatOptions, type GenerateWebfontsInputOptions, type GlyphChangeEntry, type HtmlContext } from '../index.js';

it('exports the public generator API', () => {
    expectTypeOf(generateWebfonts).toBeFunction();
    expectTypeOf(templates).toEqualTypeOf<{ html: string; css: string; scss: string }>();
    expectTypeOf<GenerateWebfontsInputOptions>().toExtend<{ dest: string; files: string[] }>();
    expectTypeOf<FormatOptions>().toExtend<{
        svg?: { centerVertically?: boolean };
        ttf?: { ts?: number };
        woff?: { metadata?: string };
        woff2?: { compressionQuality?: number };
    }>();
    expectTypeOf<GlyphChangeEntry>().toEqualTypeOf<{
        path: string;
        changeType: 'added' | 'changed' | 'removed';
        name?: string;
    }>();
});

it('narrows generated formats from the input', async () => {
    const result = await generateWebfonts({
        dest: 'fonts',
        files: ['icon.svg'],
        types: ['svg', 'woff2'],
        order: ['woff2', 'svg'],
        rename(name) {
            expectTypeOf(name).toBeString();
            return name;
        },
        cssContext(context) {
            expectTypeOf(context).toEqualTypeOf<CssContext>();
        },
        htmlContext(context) {
            expectTypeOf(context).toEqualTypeOf<HtmlContext>();
        },
    });

    expectTypeOf(result.svg).toBeString();
    expectTypeOf(result.woff2).toEqualTypeOf<Uint8Array>();
    expectTypeOf(result.eot).toBeNull();
    expectTypeOf(result.ttf).toBeNull();
    expectTypeOf(result.woff).toBeNull();
    expectTypeOf(result.generateCss({ svg: '/font.svg' })).toBeString();
    expectTypeOf(result.generateHtml()).toBeString();
    expectTypeOf(result.regenerateAsync(['icon.svg'])).resolves.toEqualTypeOf<typeof result>();
});

it('rejects invalid format combinations and callbacks', () => {
    expectTypeOf(generateWebfonts).toBeFunction();

    void generateWebfonts({
        dest: 'fonts',
        files: ['icon.svg'],
        types: ['woff2'],
        // @ts-expect-error order is restricted to selected formats
        order: ['svg'],
    });

    void generateWebfonts({
        dest: 'fonts',
        files: ['icon.svg'],
        // @ts-expect-error rename must return a glyph name
        rename: () => 1,
    });
});

it('keeps the generated NAPI declarations compatible', () => {
    expectTypeOf(generateNativeWebfonts).parameters.toEqualTypeOf<
        [
            options: import('../binding.js').GenerateWebfontsOptions,
            rename?: ((paths: string[]) => string[]) | null,
            cssContext?: ((context: Record<string, any>) => Record<string, any>) | null,
            htmlContext?: ((context: Record<string, any>) => Record<string, any>) | null,
        ]
    >();
    expectTypeOf<NativeGenerateWebfontsResult['svg']>().toEqualTypeOf<string | null>();
    expectTypeOf<NativeGenerateWebfontsResult['woff2']>().toEqualTypeOf<Uint8Array | null>();
    expectTypeOf<NativeGenerateWebfontsResult['regenerate']>().parameters.toEqualTypeOf<[files: string[], changes?: GlyphChangeEntry[] | null]>();
    expectTypeOf<NativeGenerateWebfontsResult['regenerateAsync']>().returns.toEqualTypeOf<Promise<NativeGenerateWebfontsResult>>();
});
