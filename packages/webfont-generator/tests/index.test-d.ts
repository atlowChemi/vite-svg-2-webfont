import { expectTypeOf, it } from 'vite-plus/test';
import { GenerateWebfontsResult as NativeGenerateWebfontsResult, generateWebfonts as generateNativeWebfonts } from '../binding.js';
import {
    generateWebfonts,
    templates,
    type CssContext,
    type FontVariant,
    type FormatOptions,
    type GenerateWebfontsBaseOptions,
    type GenerateWebfontsFileOptions,
    type GenerateWebfontsInputOptions,
    type GenerateWebfontsOptions,
    type GenerateWebfontsResult,
    type GenerateWebfontsVariantOptions,
    type GlyphChangeEntry,
    type HtmlContext,
    MissingGlyphBehavior,
    type MissingGlyphOptions,
} from '../index.js';

it('exports the public generator API', () => {
    expectTypeOf(generateWebfonts).toBeFunction();
    expectTypeOf(templates).toEqualTypeOf<{ html: string; css: string; scss: string }>();
    expectTypeOf(MissingGlyphBehavior).toEqualTypeOf<{
        readonly Blank: 'blank';
        readonly Error: 'error';
        readonly Fallback: 'fallback';
    }>();
    expectTypeOf<GenerateWebfontsFileOptions>().toExtend<GenerateWebfontsBaseOptions & { files: string[] }>();
    expectTypeOf<GenerateWebfontsVariantOptions>().toExtend<GenerateWebfontsBaseOptions & { variants: FontVariant[] }>();
    expectTypeOf<{ dest: string }>().not.toExtend<GenerateWebfontsInputOptions>();
    expectTypeOf<{ dest: string; files: string[]; variants: FontVariant[] }>().not.toExtend<GenerateWebfontsInputOptions>();
    expectTypeOf<GenerateWebfontsOptions>().toEqualTypeOf<GenerateWebfontsInputOptions>();
    expectTypeOf<{ dest: string; incremental: true; variants: FontVariant[] }>().not.toExtend<GenerateWebfontsVariantOptions>();
    expectTypeOf<{ dest: string; types: ['svg']; variants: FontVariant[] }>().not.toExtend<GenerateWebfontsVariantOptions>();
    expectTypeOf<FontVariant>().toExtend<{ name: string; files: string[]; weight?: number; default?: boolean }>();
    expectTypeOf<MissingGlyphOptions>().toEqualTypeOf<{ behavior: MissingGlyphBehavior; variant?: string }>();
    expectTypeOf<keyof FormatOptions>().toEqualTypeOf<'svg' | 'ttf' | 'woff' | 'woff2'>();
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

it('accepts the multi-variant contract', () => {
    expectTypeOf(
        generateWebfonts({
            dest: 'fonts',
            types: ['woff2'],
            variants: [
                { default: true, files: ['small.svg'], name: 'small', weight: 300 },
                { files: ['large.svg'], name: 'large', weight: 700 },
            ],
            missingGlyphs: {
                behavior: MissingGlyphBehavior.Fallback,
                variant: 'small',
            },
        }),
    ).toEqualTypeOf<Promise<GenerateWebfontsResult<'woff2'>>>();
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
    expectTypeOf<{ dest: string; files: string[]; order: ['svg']; types: ['woff2'] }>().not.toExtend<GenerateWebfontsFileOptions<'woff2'>>();
    expectTypeOf<{ dest: string; files: string[]; rename: () => number }>().not.toExtend<GenerateWebfontsFileOptions>();
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
