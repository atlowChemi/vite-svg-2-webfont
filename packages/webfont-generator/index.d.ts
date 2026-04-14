import type { GenerateWebfontsOptions, GenerateWebfontsResult as RawGenerateWebfontsResult } from './binding';
import * as templates from './templates.js';

export type FontType = 'svg' | 'ttf' | 'eot' | 'woff' | 'woff2';

export interface GenerateWebfontsInputOptions<T extends FontType = FontType> extends Omit<GenerateWebfontsOptions, 'types' | 'order'> {
    cssContext?: (context: Record<string, any>) => void;
    htmlContext?: (context: Record<string, any>) => void;
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
    GenerateWebfontsOptions,
    RawGenerateWebfontsResult,
    /**
     * Paths of default templates available for use.
     */
    templates,
};
