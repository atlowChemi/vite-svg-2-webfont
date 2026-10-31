declare module 'virtual:group-icons.css';

declare module '*.css' {
    const content: Record<string, string>;
    export default content;
}

declare module '*.vue' {
    import type { DefineComponent } from 'vue';
    const component: DefineComponent<{}, {}, unknown>;
    export default component;
}

declare module 'virtual:svg-authoring-fonts';

declare module 'virtual:docs-examples';

// opentype.js 2 ships no type declarations; scripts/generate-brand.ts only needs this much.
declare module 'opentype.js' {
    interface BoundingBox {
        x1: number;
        y1: number;
        x2: number;
        y2: number;
    }
    type PathCommand =
        | { type: 'M' | 'L'; x: number; y: number }
        | { type: 'Q'; x1: number; y1: number; x: number; y: number }
        | { type: 'C'; x1: number; y1: number; x2: number; y2: number; x: number; y: number }
        | { type: 'Z' };
    export interface Path {
        commands: PathCommand[];
        getBoundingBox(): BoundingBox;
    }
    interface Font {
        getPath(text: string, x: number, y: number, fontSize: number, options?: { kerning?: boolean }): Path;
        getAdvanceWidth(text: string, fontSize: number, options?: { kerning?: boolean }): number;
    }
    const opentype: { parse(buffer: ArrayBufferLike): Font };
    export default opentype;
}

declare module 'virtual:docs-examples-css' {
    const css: Record<string, string>;
    export default css;
}
