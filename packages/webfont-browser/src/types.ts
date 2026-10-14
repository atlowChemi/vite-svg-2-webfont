/** Supported font containers. Variant families support TTF, WOFF and WOFF2. */
export type FontType = 'svg' | 'ttf' | 'eot' | 'woff' | 'woff2';
/** An SVG document with an opaque cache ID and a Unicode alphanumeric/hyphen/underscore glyph name. */
export type SvgSource = { id: string; name: string; svg: string };
export type SourceFontVariant = { name: string; sources: SvgSource[]; weight?: number; default?: boolean };
export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

/** In-memory engine settings. Templates contain text, never filenames. */
export type GenerateWebfontsOptions = {
    sources?: SvgSource[];
    variants?: SourceFontVariant[];
    cssTemplate?: string;
    htmlTemplate?: string;
    colorGlyphs?: true | string[];
    ascent?: number;
    centerHorizontally?: boolean;
    centerVertically?: boolean;
    css?: boolean;
    codepoints?: Record<string, number>;
    cssFontsUrl?: string;
    descent?: number;
    fixedWidth?: boolean;
    formatOptions?: {
        svg?: { centerVertically?: boolean; fontId?: string; metadata?: string; optimizeOutput?: boolean; preserveAspectRatio?: boolean };
        /** Unix seconds. Omission captures the browser clock once; set 0 for reproducible output. */
        ttf?: { copyright?: string; description?: string; ts?: number; url?: string; version?: string };
        woff?: { metadata?: string };
        woff2?: { compressionQuality?: number };
    };
    html?: boolean;
    incremental?: boolean;
    fontHeight?: number;
    fontName?: string;
    fontStyle?: string;
    fontWeight?: string;
    ligature?: boolean;
    missingGlyphs?: { behavior: 'blank' | 'error' | 'fallback'; variant?: string };
    normalize?: boolean;
    order?: FontType[];
    optimizeOutput?: boolean;
    preserveAspectRatio?: boolean;
    round?: number;
    startCodepoint?: number;
    templateOptions?: Record<string, JsonValue>;
    types?: FontType[];
    variantClassPrefix?: string;
};

/** Complete membership after an update. Variants must retain their configured order. */
export type SourceMembership = { sources: string[]; variants?: never } | { variants: { name: string; ids: string[] }[]; sources?: never };
export type SourceChange = ({ type: 'added' } & SvgSource) | { type: 'changed'; id: string; svg: string; name?: string } | { type: 'removed'; id: string };
/** Bytes are independently owned and remain valid after regeneration or disposal. */
export type CollectedOutput = { kind: FontType | 'css' | 'html'; bytes: Uint8Array<ArrayBuffer> };

/** One worker-owned result. Calls execute in submission order, including after recoverable errors. */
export type WebfontResult = {
    font: (format: FontType) => Promise<Uint8Array<ArrayBuffer> | undefined>;
    generateCss: () => Promise<string>;
    generateHtml: () => Promise<string>;
    collectOutputs: () => Promise<CollectedOutput[]>;
    regenerate: (membership: SourceMembership, changes: SourceChange[]) => Promise<void>;
    /** Terminate the worker and reject outstanding calls. Safe to call repeatedly. */
    dispose: () => void;
};
