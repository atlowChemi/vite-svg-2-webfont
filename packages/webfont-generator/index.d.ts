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

export interface GenerateWebfontsInputOptions<T extends FontType = FontType> extends Omit<GenerateWebfontsOptions, 'types' | 'order'> {
    cssContext?: (context: CssContext) => void;
    htmlContext?: (context: HtmlContext) => void;
    order?: NoInfer<T>[];
    rename?: (name: string) => string;
    types?: T[];
}

type FontValue<F extends FontType> = F extends 'svg' ? string : Uint8Array;

export type GenerateWebfontsResult<T extends FontType = FontType> = {
    [F in FontType]: F extends T ? FontValue<F> : null;
} & Pick<RawGenerateWebfontsResult, 'generateCss' | 'generateHtml'>;

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
