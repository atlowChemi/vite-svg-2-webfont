import { expectTypeOf, it } from 'vite-plus/test';
import { GenerateWebfontsResult as NativeGenerateWebfontsResult, generateWebfonts as generateNativeWebfonts } from '../binding.js';
import {
    generateWebfonts,
    templates,
    type CssContext,
    type FontVariant,
    type FormatOptions,
    type FontType,
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
    type RegenerationFiles,
    type VariantFileSet,
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
    expectTypeOf<GenerateWebfontsVariantOptions>().toExtend<GenerateWebfontsBaseOptions<true> & { variants: FontVariant[] }>();
    expectTypeOf<{ dest: string }>().not.toExtend<GenerateWebfontsInputOptions>();
    expectTypeOf<{ dest: string; files: string[]; variants: FontVariant[] }>().not.toExtend<GenerateWebfontsInputOptions>();
    expectTypeOf<GenerateWebfontsOptions>().toEqualTypeOf<GenerateWebfontsInputOptions>();
    expectTypeOf<{ dest: string; incremental: true; variants: FontVariant[] }>().toExtend<GenerateWebfontsVariantOptions>();
    expectTypeOf<{ dest: string; incremental: false; variants: FontVariant[] }>().toExtend<GenerateWebfontsVariantOptions>();
    expectTypeOf<{ dest: string; types: ['svg']; variants: FontVariant[] }>().not.toExtend<GenerateWebfontsVariantOptions>();
    expectTypeOf<{ dest: string; types: ['eot']; variants: FontVariant[] }>().not.toExtend<GenerateWebfontsVariantOptions>();
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
    expectTypeOf(result.regenerateAsync({ files: ['icon.svg'] })).resolves.toEqualTypeOf<typeof result>();
});

it('accepts variant regeneration inputs and preserves exact output formats', async () => {
    const result = await generateWebfonts({
        dest: 'fonts',
        incremental: true,
        types: ['ttf', 'woff2'],
        variants: [
            { name: 'light', files: ['light/add.svg'], default: true },
            { name: 'bold', files: ['bold/add.svg'] },
        ],
    });
    const files = {
        variants: [
            { variant: 'light', files: ['light/add.svg'] },
            { variant: 'bold', files: ['bold/add.svg'] },
        ],
    };
    const changes = [{ path: 'bold/add.svg', changeType: 'changed' as const, name: 'add' }];
    expectTypeOf(result.regenerate(files, changes)).toBeVoid();
    expectTypeOf(result.regenerate(files)).toBeVoid();
    expectTypeOf(result.regenerate(files, null)).toBeVoid();
    expectTypeOf(result.regenerateAsync(files, changes)).toEqualTypeOf<Promise<GenerateWebfontsResult<'ttf' | 'woff2'>>>();
    expectTypeOf(result.regenerateAsync(files)).toEqualTypeOf<Promise<GenerateWebfontsResult<'ttf' | 'woff2'>>>();
    expectTypeOf(result.regenerateAsync(files, null)).toEqualTypeOf<Promise<GenerateWebfontsResult<'ttf' | 'woff2'>>>();
    const replacement = await result.regenerateAsync(files, changes);
    expectTypeOf(replacement.ttf).toEqualTypeOf<Uint8Array>();
    expectTypeOf(replacement.woff2).toEqualTypeOf<Uint8Array>();
    expectTypeOf(replacement.woff).toBeNull();
    expectTypeOf(replacement.svg).toBeNull();
    expectTypeOf(replacement.eot).toBeNull();
});

it('preserves guaranteed and nullable formats through variant regeneration', async () => {
    const selected: ['woff' | 'woff2', 'ttf'] = ['woff2', 'ttf'];
    const result = await generateWebfonts({ dest: 'fonts', incremental: true, variants: [], types: selected });
    const files = { variants: [{ variant: 'light', files: ['light/add.svg'] }] };
    expectTypeOf(result.regenerate(files, [])).toBeVoid();
    expectTypeOf(result.regenerateAsync(files, [])).toEqualTypeOf<Promise<GenerateWebfontsResult<'ttf' | 'woff' | 'woff2', 'ttf'>>>();
    const replacement = await result.regenerateAsync(files);
    expectTypeOf(replacement.ttf).toEqualTypeOf<Uint8Array>();
    expectTypeOf(replacement.woff).toEqualTypeOf<Uint8Array | null>();
    expectTypeOf(replacement.woff2).toEqualTypeOf<Uint8Array | null>();
    const types: Array<'woff' | 'woff2'> = ['woff2'];
    const dynamic = await generateWebfonts({ dest: 'fonts', incremental: true, variants: [], types });
    expectTypeOf(dynamic.regenerateAsync(files)).toEqualTypeOf<Promise<GenerateWebfontsResult<'woff' | 'woff2', never>>>();
});

it('fixes the regeneration source contract and rejects malformed variant inputs', () => {
    type ExpectedFiles = { files: string[]; variants?: never } | { files?: never; variants: { variant: string; files: string[] }[] };
    type ExpectedChanges = { path: string; changeType: 'added' | 'changed' | 'removed'; name?: string }[];
    expectTypeOf<VariantFileSet>().toEqualTypeOf<{ variant: string; files: string[] }>();
    expectTypeOf<RegenerationFiles>().toEqualTypeOf<ExpectedFiles>();
    expectTypeOf<GenerateWebfontsResult['regenerate']>().parameters.toEqualTypeOf<[files: ExpectedFiles, changes?: ExpectedChanges | null]>();
    expectTypeOf<GenerateWebfontsResult['regenerateAsync']>().parameters.toEqualTypeOf<[files: ExpectedFiles, changes?: ExpectedChanges | null]>();
    const result = {} as GenerateWebfontsResult<'woff2'>;
    // @ts-expect-error Each design requires its variant name.
    result.regenerate({ variants: [{ files: ['add.svg'] }] });
    // @ts-expect-error Each design requires its complete file list.
    void result.regenerateAsync({ variants: [{ variant: 'bold' }] });
    // @ts-expect-error Variant file lists contain paths, not numeric IDs.
    result.regenerate({ variants: [{ variant: 'bold', files: [1] }] });
    // @ts-expect-error The source modes are mutually exclusive.
    result.regenerate({ files: [], variants: [] });
    // @ts-expect-error The async method also rejects mixed source modes.
    void result.regenerateAsync({ files: [], variants: [] });
    // @ts-expect-error Change kinds retain their literal union for variant inputs.
    result.regenerate({ variants: [] }, [{ path: 'add.svg', changeType: 'renamed' }]);
    // @ts-expect-error Async change kinds retain their literal union too.
    void result.regenerateAsync({ variants: [] }, [{ path: 'add.svg', changeType: 'renamed' }]);
});

it('uses runtime format defaults when types are omitted', async () => {
    const ordinary = await generateWebfonts({ dest: 'fonts', files: ['icon.svg'] });
    expectTypeOf(ordinary.svg).toBeNull();
    expectTypeOf(ordinary.ttf).toBeNull();
    expectTypeOf(ordinary.eot).toEqualTypeOf<Uint8Array>();
    expectTypeOf(ordinary.woff).toEqualTypeOf<Uint8Array>();
    expectTypeOf(ordinary.woff2).toEqualTypeOf<Uint8Array>();
    const variant = await generateWebfonts({ dest: 'fonts', variants: [] });
    expectTypeOf(variant.svg).toBeNull();
    expectTypeOf(variant.eot).toBeNull();
    expectTypeOf(variant.ttf).toBeNull();
    expectTypeOf(variant.woff).toEqualTypeOf<Uint8Array>();
    expectTypeOf(variant.woff2).toEqualTypeOf<Uint8Array>();
});

it('preserves explicit single-format generics without overpromising dynamic selections', async () => {
    const svg = await generateWebfonts<'svg'>({ dest: 'fonts', files: ['icon.svg'], types: ['svg'] });
    expectTypeOf(svg).toEqualTypeOf<GenerateWebfontsResult<'svg'>>();
    expectTypeOf(svg.svg).toBeString();
    const modern = await generateWebfonts<'woff2'>({ dest: 'fonts', variants: [], types: ['woff2'] });
    expectTypeOf(modern).toEqualTypeOf<GenerateWebfontsResult<'woff2'>>();
    expectTypeOf(modern.woff2).toEqualTypeOf<Uint8Array>();
    const types: Array<'svg'> = [];
    const dynamic = await generateWebfonts<'svg'>({ dest: 'fonts', files: ['icon.svg'], types });
    expectTypeOf(dynamic.svg).toEqualTypeOf<string | null>();
    const possible = await generateWebfonts<'svg' | 'woff2'>({ dest: 'fonts', files: ['icon.svg'], types: ['svg'] });
    expectTypeOf(possible.svg).toEqualTypeOf<string | null>();
    expectTypeOf(possible.woff2).toEqualTypeOf<Uint8Array | null>();
    const omitted = await generateWebfonts<'svg'>({ dest: 'fonts', files: ['icon.svg'] });
    expectTypeOf(omitted.svg).toEqualTypeOf<string | null>();
    // @ts-expect-error Explicit generics cannot enable SVG for variants.
    void generateWebfonts<'svg'>({ dest: 'fonts', variants: [], types: ['svg'] });
});

it('keeps dynamic format selections nullable and accepts options unions', async () => {
    const types: FontType[] = ['woff2'];
    const dynamic = await generateWebfonts({ dest: 'fonts', files: ['icon.svg'], types });
    expectTypeOf(dynamic.svg).toEqualTypeOf<string | null>();
    expectTypeOf(dynamic.ttf).toEqualTypeOf<Uint8Array | null>();
    expectTypeOf(dynamic.regenerateAsync({ files: [] })).resolves.toEqualTypeOf<typeof dynamic>();
    const narrowTypes: Array<'woff' | 'woff2'> = ['woff2'];
    const variants = await generateWebfonts({ dest: 'fonts', variants: [], types: narrowTypes });
    expectTypeOf(variants.ttf).toBeNull();
    expectTypeOf(variants.woff).toEqualTypeOf<Uint8Array | null>();
    expectTypeOf(variants.woff2).toEqualTypeOf<Uint8Array | null>();
    const selected: ['woff' | 'woff2', 'ttf'] = ['woff2', 'ttf'];
    const selectedResult = await generateWebfonts({ dest: 'fonts', variants: [], types: selected });
    expectTypeOf(selectedResult.woff).toEqualTypeOf<Uint8Array | null>();
    expectTypeOf(selectedResult.woff2).toEqualTypeOf<Uint8Array | null>();
    expectTypeOf(selectedResult.ttf).toEqualTypeOf<Uint8Array>();
    const options = {} as GenerateWebfontsOptions;
    const result = await generateWebfonts(options);
    expectTypeOf(result.svg).toEqualTypeOf<string | null>();
    expectTypeOf(result.woff2).toEqualTypeOf<Uint8Array | null>();
});

it('types resolved variant template metadata without casts', () => {
    expectTypeOf<CssContext<true>['variants']>().toEqualTypeOf<HtmlContext<true>['variants']>();
    expectTypeOf<CssContext<true>['variants'][number]>().toEqualTypeOf<{
        name: string;
        weight: number;
        default: boolean;
        className: string;
        selector: string;
    }>();
    expectTypeOf<CssContext<true>['defaultWeight']>().toEqualTypeOf<number>();
    expectTypeOf<CssContext<true>['fontStyle']>().toEqualTypeOf<string>();
    expectTypeOf<HtmlContext<true>['variantClassPrefix']>().toEqualTypeOf<string>();
    void generateWebfonts({
        dest: 'fonts',
        variants: [],
        cssContext(context) {
            expectTypeOf(context).toEqualTypeOf<CssContext<true>>();
            context.variants[0].weight.toFixed();
        },
        htmlContext(context) {
            expectTypeOf(context).toEqualTypeOf<HtmlContext<true>>();
            context.defaultWeight.toFixed();
        },
    });
});

it('keeps ordinary and unspecified variant metadata unknown', () => {
    type MetadataKey = 'variants' | 'defaultWeight' | 'fontStyle' | 'variantClassPrefix';
    expectTypeOf<CssContext[MetadataKey]>().toBeUnknown();
    expectTypeOf<HtmlContext[MetadataKey]>().toBeUnknown();
    expectTypeOf<import('../binding.js').CssContext[MetadataKey]>().toBeUnknown();
    expectTypeOf<import('../binding.js').HtmlContext[MetadataKey]>().toBeUnknown();
    void generateWebfonts({
        dest: 'fonts',
        files: ['icon.svg'],
        templateOptions: { variants: [{ name: 'unrelated' }] },
        cssContext(context) {
            expectTypeOf(context.variants).toBeUnknown();
            // @ts-expect-error Ordinary template data does not guarantee a weight.
            context.variants?.[0].weight.toFixed();
        },
        htmlContext(context) {
            expectTypeOf(context.defaultWeight).toBeUnknown();
            // @ts-expect-error Ordinary template data does not guarantee a numeric default.
            context.defaultWeight.toFixed();
        },
    });
});

it('rejects invalid format combinations and callbacks', () => {
    expectTypeOf(generateWebfonts).toBeFunction();
    expectTypeOf<{ dest: string; files: string[]; order: ['svg']; types: ['woff2'] }>().not.toExtend<GenerateWebfontsFileOptions<'woff2'>>();
    expectTypeOf<{ dest: string; files: string[]; rename: () => number }>().not.toExtend<GenerateWebfontsFileOptions>();
    // @ts-expect-error Variant generation does not accept legacy formats.
    void generateWebfonts({ dest: 'fonts', variants: [], types: ['svg'] });
    // @ts-expect-error The options-union overload must still reject mixed sources.
    void generateWebfonts({ dest: 'fonts', files: ['icon.svg'], variants: [] });
    // @ts-expect-error Order must be a subset of the requested formats.
    void generateWebfonts({ dest: 'fonts', files: ['icon.svg'], types: ['woff2'], order: ['svg'] });
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
    type ExpectedNativeParameters = [
        files: { files?: string[]; variants?: { variant: string; files: string[] }[] },
        changes?: { path: string; changeType: 'added' | 'changed' | 'removed'; name?: string }[] | null,
    ];
    expectTypeOf<NativeGenerateWebfontsResult['regenerate']>().parameters.toEqualTypeOf<ExpectedNativeParameters>();
    expectTypeOf<NativeGenerateWebfontsResult['regenerate']>().returns.toBeVoid();
    expectTypeOf<NativeGenerateWebfontsResult['regenerateAsync']>().parameters.toEqualTypeOf<ExpectedNativeParameters>();
    expectTypeOf<NativeGenerateWebfontsResult['regenerateAsync']>().returns.toEqualTypeOf<Promise<NativeGenerateWebfontsResult>>();
});
