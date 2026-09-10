import type {
    CssContext as RawCssContext,
    FontVariant,
    FormatOptions,
    GenerateWebfontsOptions as RawGenerateWebfontsOptions,
    GenerateWebfontsResult as RawGenerateWebfontsResult,
    GlyphChangeEntry,
    HtmlContext as RawHtmlContext,
    SvgFormatOptions,
    TemplateVariant,
    TtfFormatOptions,
    Woff2FormatOptions,
    WoffFormatOptions,
} from './binding';
import * as templates from './templates.js';

/**
 * Font output format. Used in the `types` and `order` options to control which
 * formats are generated and the order they appear in the CSS `@font-face`
 * `src:` descriptor.
 */
export type FontType = 'svg' | 'ttf' | 'eot' | 'woff' | 'woff2';
export type MultiVariantFontType = Exclude<FontType, 'svg' | 'eot'>;

export declare const MissingGlyphBehavior: {
    readonly Blank: 'blank';
    readonly Error: 'error';
    readonly Fallback: 'fallback';
};
export type MissingGlyphBehavior = (typeof MissingGlyphBehavior)[keyof typeof MissingGlyphBehavior];
export interface MissingGlyphOptions {
    behavior: MissingGlyphBehavior;
    variant?: string;
}

/**
 * Context object passed to the `cssContext` callback. The named fields are
 * supplied by the native engine; variant fields are optional in ordinary mode. The index signature accommodates
 * arbitrary keys merged in from user-supplied `templateOptions`.
 */
export type CssContext = RawCssContext & { [key: string]: unknown };

/**
 * Context object passed to the `htmlContext` callback. The named fields are
 * supplied by the native engine; variant fields are optional in ordinary mode. The index signature accommodates
 * arbitrary keys merged in from user-supplied `templateOptions`.
 */
export type HtmlContext = RawHtmlContext & { [key: string]: unknown };

/** Options shared by ordinary and multi-variant generation. */
export interface GenerateWebfontsBaseOptions extends Omit<
    RawGenerateWebfontsOptions,
    'files' | 'fontWeight' | 'incremental' | 'missingGlyphs' | 'order' | 'types' | 'variantClassPrefix' | 'variants'
> {
    /**
     * Mutate the Handlebars context before CSS rendering. Modify `context`
     * in-place; the return value is ignored.
     */
    cssContext?: (context: CssContext) => void;
    /**
     * Mutate the Handlebars context before HTML preview rendering. Modify
     * `context` in-place; the return value is ignored.
     */
    htmlContext?: (context: HtmlContext) => void;
    /**
     * Derive a custom glyph name from each SVG file path. Receives the file
     * path; must return the glyph name.
     */
    rename?: (name: string) => string;
}

/** Generate one ordinary font from a `files` source. */
export interface GenerateWebfontsFileOptions<T extends FontType = FontType> extends GenerateWebfontsBaseOptions {
    files: string[];
    fontWeight?: RawGenerateWebfontsOptions['fontWeight'];
    incremental?: RawGenerateWebfontsOptions['incremental'];
    missingGlyphs?: never;
    order?: NoInfer<T>[];
    types?: T[];
    variantClassPrefix?: never;
    variants?: never;
}

/**
 * Generate one shared variable TTF/WOFF/WOFF2 per requested format.
 */
export interface GenerateWebfontsVariantOptions<T extends MultiVariantFontType = MultiVariantFontType> extends GenerateWebfontsBaseOptions {
    files?: never;
    fontWeight?: never;
    incremental?: false;
    missingGlyphs?: MissingGlyphOptions;
    order?: NoInfer<T>[];
    types?: T[];
    variantClassPrefix?: string;
    variants: FontVariant[];
}

/**
 * Options accepted by `generateWebfonts`. Exactly one source, `files` or
 * `variants`, is required. Inferring `T` from `types` narrows the generated
 * font properties on the result.
 */
export type GenerateWebfontsInputOptions<T extends FontType = FontType> = GenerateWebfontsFileOptions<T> | GenerateWebfontsVariantOptions<Extract<T, MultiVariantFontType>>;
export type GenerateWebfontsOptions<T extends FontType = FontType> = GenerateWebfontsInputOptions<T>;

type FontValue<F extends FontType> = F extends 'svg' ? string : Uint8Array;

type ContainsFormat<Formats extends readonly FontType[], F extends FontType> = Formats extends readonly [infer Head, ...infer Tail extends FontType[]]
    ? [Head] extends [F]
        ? true
        : ContainsFormat<Tail, F>
    : false;
type GuaranteedFormats<Formats extends readonly FontType[]> = {
    [F in Formats[number]]: [ContainsFormat<Formats, F>] extends [true] ? F : never;
}[Formats[number]];

/**
 * Result of a successful `generateWebfonts` call. Each font format is exposed
 * as a property. `T` is the set of possible formats; `Guaranteed` is the set
 * known to be requested. Possible but unconfirmed formats are nullable;
 * excluded formats are `null`. Literal format lists and omitted `types`
 * infer guaranteed formats, while widened arrays retain nullable getters.
 *
 * Also carries `generateCss` and `generateHtml` for rendering with custom
 * URLs after the fact.
 */
export type GenerateWebfontsResult<T extends FontType = FontType, Guaranteed extends T = T> = {
    [F in FontType]: F extends Guaranteed ? FontValue<F> : F extends T ? FontValue<F> | null : null;
} & Pick<RawGenerateWebfontsResult, 'generateCss' | 'generateHtml' | 'regenerate'> & {
        regenerateAsync(files: string[], changes?: GlyphChangeEntry[] | null): Promise<GenerateWebfontsResult<T, Guaranteed>>;
    };

/**
 * Generate a webfont from ordinary SVG files or ordered multi-weight variants.
 *
 * Ordinary generation loads `options.files`, builds the configured formats,
 * optionally writes them to `options.dest`, and resolves with the font bytes
 * and template-rendering methods. Multi-variant generation returns shared modern
 * fonts through the same getters; SVG and EOT are unsupported in variant mode.
 */
export declare function generateWebfonts<const Formats extends readonly FontType[]>(
    options: Omit<GenerateWebfontsFileOptions<Formats[number]>, 'types'> & { types: Formats },
): Promise<GenerateWebfontsResult<Formats[number], number extends Formats['length'] ? never : GuaranteedFormats<Formats>>>;
export declare function generateWebfonts<const Formats extends readonly MultiVariantFontType[]>(
    options: Omit<GenerateWebfontsVariantOptions<Formats[number]>, 'types'> & { types: Formats },
): Promise<GenerateWebfontsResult<Formats[number], number extends Formats['length'] ? never : GuaranteedFormats<Formats>>>;
export declare function generateWebfonts(
    options: GenerateWebfontsFileOptions<'eot' | 'woff' | 'woff2'> & { types?: undefined },
): Promise<GenerateWebfontsResult<'eot' | 'woff' | 'woff2'>>;
export declare function generateWebfonts(options: GenerateWebfontsVariantOptions<'woff' | 'woff2'> & { types?: undefined }): Promise<GenerateWebfontsResult<'woff' | 'woff2'>>;
/** Options whose format selection is not statically known have nullable getters. */
export declare function generateWebfonts<T extends FontType = FontType>(
    options: GenerateWebfontsFileOptions<T>,
): Promise<GenerateWebfontsResult<T | 'eot' | 'woff' | 'woff2', never>>;
export declare function generateWebfonts<T extends MultiVariantFontType = MultiVariantFontType>(
    options: GenerateWebfontsVariantOptions<T>,
): Promise<GenerateWebfontsResult<T | 'woff' | 'woff2', never>>;
export declare function generateWebfonts<T extends FontType = FontType>(
    options: GenerateWebfontsInputOptions<T>,
): Promise<GenerateWebfontsResult<T | 'eot' | 'woff' | 'woff2', never>>;

export declare namespace generateWebfonts {
    /**
     * Paths of default templates available for use.
     */
    const templates: typeof import('./templates.js');
}

export {
    FormatOptions,
    FontVariant,
    GlyphChangeEntry,
    RawGenerateWebfontsResult,
    SvgFormatOptions,
    TemplateVariant,
    /**
     * Paths of default templates available for use.
     */
    templates,
    TtfFormatOptions,
    Woff2FormatOptions,
    WoffFormatOptions,
};
