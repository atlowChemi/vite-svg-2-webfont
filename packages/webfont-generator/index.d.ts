import type {
    CssContext as RawCssContext,
    FormatOptions,
    GenerateWebfontsOptions,
    GenerateWebfontsResult as RawGenerateWebfontsResult,
    HtmlContext as RawHtmlContext,
    SvgFormatOptions,
    TtfFormatOptions,
    WoffFormatOptions,
} from './binding';
import * as templates from './templates.js';

/**
 * Font output format. Used in the `types` and `order` options to control which
 * formats are generated and the order they appear in the CSS `@font-face`
 * `src:` descriptor.
 */
export type FontType = 'svg' | 'ttf' | 'eot' | 'woff' | 'woff2';

/**
 * Context object passed to the `cssContext` callback. The named fields are
 * always supplied by the native engine; the index signature accommodates
 * arbitrary keys merged in from user-supplied `templateOptions`.
 */
export type CssContext = RawCssContext & { [key: string]: unknown };

/**
 * Context object passed to the `htmlContext` callback. The named fields are
 * always supplied by the native engine; the index signature accommodates
 * arbitrary keys merged in from user-supplied `templateOptions`.
 */
export type HtmlContext = RawHtmlContext & { [key: string]: unknown };

/**
 * Options accepted by `generateWebfonts`. Extends the native
 * `GenerateWebfontsOptions` with JS-only callbacks (`cssContext`,
 * `htmlContext`, `rename`) and narrows `types`/`order` so the resolved result
 * type only includes the requested formats.
 *
 * Inferring `T` from `types` lets the returned `GenerateWebfontsResult` know
 * exactly which font properties are non-null.
 */
export interface GenerateWebfontsInputOptions<T extends FontType = FontType> extends Omit<GenerateWebfontsOptions, 'types' | 'order'> {
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
     * Order of `@font-face` `src:` entries in the generated CSS. Every entry
     * must also appear in `types`. Defaults to
     * `['eot', 'woff2', 'woff', 'ttf', 'svg']` filtered to the requested
     * `types`.
     */
    order?: NoInfer<T>[];
    /**
     * Derive a custom glyph name from each SVG file path. Receives the file
     * path; must return the glyph name.
     */
    rename?: (name: string) => string;
    /** Font formats to generate. Defaults to `['eot', 'woff', 'woff2']`. */
    types?: T[];
}

type FontValue<F extends FontType> = F extends 'svg' ? string : Uint8Array;

/**
 * Result of a successful `generateWebfonts` call. Each font format is exposed
 * as a property — formats included in `types` carry their bytes (or, for
 * `svg`, the XML string), and formats not in `types` are typed as `null`.
 *
 * Also carries `generateCss` and `generateHtml` for rendering with custom
 * URLs after the fact.
 */
export type GenerateWebfontsResult<T extends FontType = FontType> = {
    [F in FontType]: F extends T ? FontValue<F> : null;
} & Pick<RawGenerateWebfontsResult, 'generateCss' | 'generateHtml'>;

/**
 * Generate a webfont from a set of SVG files.
 *
 * Loads the SVGs listed in `options.files`, builds the configured
 * `options.types` formats, optionally writes them (along with the CSS and
 * HTML preview) to `options.dest`, and resolves to a `GenerateWebfontsResult`
 * holding the font bytes and template-rendering methods.
 */
export declare function generateWebfonts<T extends FontType = FontType>(options: GenerateWebfontsInputOptions<T>): Promise<GenerateWebfontsResult<T>>;

export declare namespace generateWebfonts {
    /**
     * Paths of default templates available for use.
     */
    const templates: typeof import('./templates.js');
}

export {
    FormatOptions,
    GenerateWebfontsOptions,
    RawGenerateWebfontsResult,
    SvgFormatOptions,
    /**
     * Paths of default templates available for use.
     */
    templates,
    TtfFormatOptions,
    WoffFormatOptions,
};
