/* tslint:disable */
/* eslint-disable */

/**
 * A retained engine result. The TypeScript API owns this object inside its worker.
 */
export class FontSession {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Append a source, requiring a target variant for families. Uses retained membership.
     */
    add(source: string, variant?: string | null): void;
    /**
     * Encode the persistent cache, or `undefined` when caching is off or nothing changed since
     * the bytes were loaded or last returned.
     */
    cacheBytes(): Uint8Array | undefined;
    /**
     * Render CSS lazily, even when CSS collection is disabled.
     */
    css(): string;
    /**
     * Copy a requested font without rendering companion templates.
     */
    font(format: string): Uint8Array | undefined;
    /**
     * Render HTML lazily, reusing the engine's CSS cache.
     */
    html(): string;
    /**
     * Decode and validate memory options. An omitted timestamp uses the browser clock,
     * captured once so incremental updates retain the same font metadata; fonts reused from
     * `cache` keep the timestamp of the session that created them. `cache` enables the
     * persistent cache with bytes from an earlier `cacheBytes()` (empty to start cold).
     */
    constructor(options: string, cache?: Uint8Array | null);
    /**
     * Copy requested fonts and enabled companions into an object keyed by output kind.
     */
    outputs(): object;
    /**
     * Apply a source batch with complete final membership. Ordinary errors retain the result.
     */
    regenerate(membership: string, changes: string): void;
    /**
     * Remove an existing source from every design, preserving the remaining order.
     */
    remove(id: string): void;
    /**
     * Replace SVG text and optionally the name of an existing source in every design.
     */
    update(id: string, svg: string, name?: string | null): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_fontsession_free: (a: number, b: number) => void;
    readonly fontsession_add: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly fontsession_cacheBytes: (a: number) => any;
    readonly fontsession_css: (a: number) => [number, number, number, number];
    readonly fontsession_font: (a: number, b: number, c: number) => [number, number, number];
    readonly fontsession_html: (a: number) => [number, number, number, number];
    readonly fontsession_new: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly fontsession_outputs: (a: number) => [number, number, number];
    readonly fontsession_regenerate: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly fontsession_remove: (a: number, b: number, c: number) => [number, number];
    readonly fontsession_update: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
