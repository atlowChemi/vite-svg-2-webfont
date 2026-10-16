import type { IconPluginFileOptions } from '../../vite-svg-2-webfont/src/index.js';

/** One live font on the SVG authoring page: a fixture folder plus the options under comparison. */
export type FontCase = { id: string; dir?: string; options?: Partial<IconPluginFileOptions> };

export const svgAuthoringFonts: FontCase[] = [
    { id: 'ink-before' },
    { id: 'ink-after', options: { colorGlyphs: ['badge'] } },
    { id: 'effects-before' },
    { id: 'effects-after' },
    { id: 'text-before' },
    { id: 'text-after' },
    { id: 'artboard-before' },
    { id: 'artboard-after' },
    { id: 'alignment-default', dir: 'alignment' },
    { id: 'alignment-no-normalize', dir: 'alignment', options: { normalize: false } },
    { id: 'alignment-center', dir: 'alignment', options: { normalize: false, centerVertically: true } },
    { id: 'alignment-fixed-width', dir: 'alignment', options: { fixedWidth: true } },
    { id: 'precision-low', dir: 'precision', options: { fontHeight: 24 } },
    { id: 'precision-default', dir: 'precision' },
    { id: 'names-before' },
    { id: 'names-after' },
    { id: 'codepoints-v1' },
    { id: 'codepoints-v2' },
    { id: 'codepoints-v2-pinned', dir: 'codepoints-v2', options: { codepoints: { add: 0xf101, remove: 0xf102 } } },
    { id: 'supported' },
];
