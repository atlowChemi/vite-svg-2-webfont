/** Supported font containers. Variant families support TTF, WOFF and WOFF2. */
export type FontType = 'svg' | 'ttf' | 'eot' | 'woff' | 'woff2';
/** An SVG document with an opaque cache ID and a Unicode alphanumeric/hyphen/underscore glyph name. */
export type SvgSource = { id: string; name: string; svg: string };
export type SourceFontVariant = { name: string; sources: SvgSource[]; weight?: number; default?: boolean };
export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

/** In-memory engine settings. Templates contain text, never filenames. */
export type GenerateWebfontsOptions = {
    /**
     * Enable the persistent cache with bytes from an earlier `exportCache()`, or an empty array to
     * start cold. Unchanged icons and formats are reused; output is byte-identical. Storing the
     * bytes is up to your app, for example in IndexedDB (they are several MB for hundreds of
     * icons). Without a fixed `formatOptions.ttf.ts`, reused fonts keep the timestamp of the
     * session that created them.
     * Single-design only; variant families ignore it.
     */
    cache?: Uint8Array;
    sources?: SvgSource[];
    variants?: SourceFontVariant[];
    cssTemplate?: string;
    htmlTemplate?: string;
    colorGlyphs?: true | string[];
    /** Named #RRGGBB palette slots, indexed in ASCII name order; requires active colorGlyphs. */
    colorSlots?: Record<string, string>;
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
export type CollectedOutputs = Partial<Record<FontType | 'css' | 'html', Uint8Array<ArrayBuffer>>>;

/** One worker-owned result. Calls execute in submission order, including after recoverable errors. */
export type WebfontResult = {
    font: (format: FontType) => Promise<Uint8Array<ArrayBuffer> | undefined>;
    generateCss: () => Promise<string>;
    generateHtml: () => Promise<string>;
    collectOutputs: () => Promise<CollectedOutputs>;
    /** Persistent cache bytes to store, or `undefined` when caching is off or nothing changed since the bytes were loaded or last exported. */
    exportCache: () => Promise<Uint8Array<ArrayBuffer> | undefined>;
    regenerate: (membership: SourceMembership, changes: SourceChange[]) => Promise<void>;
    /** Append a new source. Families require a target variant. Requires incremental generation. */
    add: (source: SvgSource, variant?: string) => Promise<void>;
    /** Replace an existing source in every consuming design, preserving its position. */
    update: (id: string, svg: string, name?: string) => Promise<void>;
    /** Remove an existing source from every design. Rejects empty designs. */
    remove: (id: string) => Promise<void>;
    /** Terminate the worker and reject outstanding calls. Safe to call repeatedly. */
    dispose: () => void;
    /** Synchronous scope cleanup for `using result = await generateWebfonts(...)`. */
    [Symbol.dispose]: () => void;
};

// Worker protocol; not exported from the package entry.
export type RequestPayload =
    | { method: 'generate'; options: string; cache?: Uint8Array }
    | { method: 'regenerate'; membership: string; changes: string }
    | { method: 'add'; source: string; variant?: string }
    | { method: 'update'; sourceId: string; svg: string; name?: string }
    | { method: 'remove'; sourceId: string }
    | { method: 'font'; format: string }
    | { method: 'css' | 'html' | 'outputs' | 'cache' };
export type Request = { id: number } & RequestPayload;
export type Result = undefined | string | Uint8Array<ArrayBuffer> | CollectedOutputs;
export type Response = { id: number } & ({ ok: true; value: Result } | { ok: false; error: Error; fatal: boolean });
