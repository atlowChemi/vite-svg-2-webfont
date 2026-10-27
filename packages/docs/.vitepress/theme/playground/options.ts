import type { FontType, GenerateWebfontsOptions, JsonValue } from '@atlowchemi/webfont-browser-generator';
import { DEFAULT_START_CODEPOINT } from './codepoints';

export type PlaygroundIcon = { id: string; name: string; svg: string; codepoint: number };

export type ColorMode = 'off' | 'all' | 'selected';

/** Every playground control. Optional numbers and empty strings mean "use the engine default". */
export type PlaygroundSettings = {
    fontName: string;
    fontStyle: string;
    fontWeight: string;
    types: FontType[];
    /** Format order in the generated `src` lists; only selected formats are used. */
    order: FontType[];
    woff2CompressionQuality: number;
    fontHeight?: number;
    ascent?: number;
    descent?: number;
    round?: number;
    normalize: boolean;
    fixedWidth: boolean;
    centerHorizontally: boolean;
    centerVertically: boolean;
    preserveAspectRatio: boolean;
    optimizeOutput: boolean;
    ligature: boolean;
    startCodepoint: number;
    css: boolean;
    html: boolean;
    ts: boolean;
    cssFontsUrl: string;
    classPrefix: string;
    baseSelector: string;
    /** Extra template data as a JSON object, merged under `classPrefix` and `baseSelector`. */
    templateData: string;
    colorMode: ColorMode;
    colorNames: string[];
    /** Custom template text; undefined uses the built-in template. */
    cssTemplate?: string;
    htmlTemplate?: string;
    svgFontId: string;
    svgMetadata: string;
    ttfCopyright: string;
    ttfDescription: string;
    ttfUrl: string;
    ttfVersion: string;
    ttfTimestamp?: number;
    woffMetadata: string;
};

export const ALL_TYPES: readonly FontType[] = ['woff2', 'woff', 'ttf', 'eot', 'svg'];

export const defaultSettings: PlaygroundSettings = {
    fontName: 'iconfont',
    fontStyle: '',
    fontWeight: '',
    types: ['woff2', 'woff'],
    order: ['eot', 'woff2', 'woff', 'ttf', 'svg'],
    woff2CompressionQuality: 11,
    // Like the Vite plugin: the engine's default (the tallest icon's height) is too coarse for 24px icons.
    fontHeight: 1000,
    normalize: true,
    fixedWidth: false,
    centerHorizontally: false,
    centerVertically: false,
    preserveAspectRatio: false,
    optimizeOutput: false,
    ligature: true,
    startCodepoint: DEFAULT_START_CODEPOINT,
    css: true,
    html: true,
    ts: false,
    cssFontsUrl: '',
    classPrefix: 'icon-',
    baseSelector: '.icon',
    templateData: '',
    colorMode: 'off',
    colorNames: [],
    svgFontId: '',
    svgMetadata: '',
    ttfCopyright: '',
    ttfDescription: '',
    ttfUrl: '',
    ttfVersion: '',
    woffMetadata: '',
};

/** Formats the current settings cannot produce: color glyphs need TTF, WOFF, or WOFF2. */
export function unavailableTypes(settings: Pick<PlaygroundSettings, 'colorMode' | 'colorNames'>): FontType[] {
    const color = settings.colorMode === 'all' || (settings.colorMode === 'selected' && settings.colorNames.length > 0);
    return color ? ['eot', 'svg'] : [];
}

export function selectedTypes(settings: PlaygroundSettings): FontType[] {
    const unavailable = unavailableTypes(settings);
    return settings.order.filter(type => settings.types.includes(type) && !unavailable.includes(type));
}

/** Parse the extra template data; throws a readable error for anything but a JSON object. */
export function parseTemplateData(text: string): Record<string, JsonValue> {
    if (!text.trim()) return {};
    let value: unknown;
    try {
        value = JSON.parse(text);
    } catch (error) {
        throw new Error(`Template data is not valid JSON: ${(error as Error).message}`, { cause: error });
    }
    if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new Error('Template data must be a JSON object');
    return value as Record<string, JsonValue>;
}

function nonEmpty(value: string): string | undefined {
    return value.trim() ? value : undefined;
}

/** Drop undefined fields, and the object itself when nothing is left. */
function compact<T extends object>(value: T): T | undefined {
    const entries = Object.entries(value).filter(([, field]) => field !== undefined);
    return entries.length ? (Object.fromEntries(entries) as T) : undefined;
}

/** The browser generator options for the playground's icons and settings. Every codepoint is passed explicitly. */
export function toGenerateOptions(icons: readonly PlaygroundIcon[], settings: PlaygroundSettings): GenerateWebfontsOptions {
    const types = selectedTypes(settings);
    const colorGlyphs = settings.colorMode === 'all' ? true : settings.colorMode === 'selected' && settings.colorNames.length ? settings.colorNames : undefined;
    return {
        sources: icons.map(({ id, name, svg }) => ({ id, name, svg })),
        codepoints: Object.fromEntries(icons.map(({ name, codepoint }) => [name, codepoint])),
        startCodepoint: settings.startCodepoint,
        fontName: settings.fontName,
        fontStyle: nonEmpty(settings.fontStyle),
        fontWeight: nonEmpty(settings.fontWeight),
        types,
        order: types,
        fontHeight: settings.fontHeight,
        ascent: settings.ascent,
        descent: settings.descent,
        round: settings.round,
        normalize: settings.normalize,
        fixedWidth: settings.fixedWidth,
        centerHorizontally: settings.centerHorizontally,
        centerVertically: settings.centerVertically,
        preserveAspectRatio: settings.preserveAspectRatio,
        optimizeOutput: settings.optimizeOutput,
        ligature: settings.ligature,
        css: settings.css,
        html: settings.html,
        ts: settings.ts,
        cssFontsUrl: nonEmpty(settings.cssFontsUrl),
        templateOptions: { ...parseTemplateData(settings.templateData), classPrefix: settings.classPrefix, baseSelector: settings.baseSelector },
        colorGlyphs,
        cssTemplate: settings.cssTemplate,
        htmlTemplate: settings.htmlTemplate,
        formatOptions: compact({
            svg: compact({ fontId: nonEmpty(settings.svgFontId), metadata: nonEmpty(settings.svgMetadata) }),
            ttf: compact({
                copyright: nonEmpty(settings.ttfCopyright),
                description: nonEmpty(settings.ttfDescription),
                url: nonEmpty(settings.ttfUrl),
                version: nonEmpty(settings.ttfVersion),
                ts: settings.ttfTimestamp,
            }),
            woff: compact({ metadata: nonEmpty(settings.woffMetadata) }),
            woff2: { compressionQuality: settings.woff2CompressionQuality },
        }),
    };
}
