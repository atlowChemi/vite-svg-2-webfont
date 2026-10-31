import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import opentype, { type Path } from 'opentype.js';

/**
 * Brand SVGs written to `assets/svg/` (the `optimize-svg` task copies them to `public/`):
 * the T mark used as favicon, and the `fonTe` wordmark for light and dark backgrounds.
 *
 * The wordmark's `fon` and `e` are Zilla Slab SemiBold outlines, so it renders identically without the web font.
 * Re-run after changing the palette or the mark: `node ./scripts/generate-brand.ts`.
 */
const outDir = resolve(import.meta.dirname, '..', 'assets', 'svg');
const fontPath = createRequire(import.meta.url).resolve('@fontsource/zilla-slab/files/zilla-slab-latin-600-normal.woff');

const palette = {
    iron: '#1c1a1f',
    ash: '#f3ece2',
    molten: '#ff6a13',
    ember: '#ffb02e',
    mold: { light: '#5a5258', dark: '#b3abba' },
};

/** The bracketed-serif T in a 100×86 box; its bottom edge sits on the text baseline. */
const T_PATH = 'M14 18 H86 V42 Q84 34 76 32 H57 V78 Q57 82 62 82 H68 V86 H32 V82 H38 Q43 82 43 78 V32 H24 Q16 34 14 42 Z';
const T_BOX = { width: 100, height: 86 };

/** A still frame of the landing page's molten surface, at the full-mold level. */
function moltenSurface(level = 22, amplitude = 1.3, phase = 0): string {
    const points: string[] = [];
    for (let x = 0; x <= 100; x += 4) {
        const y = level + amplitude * Math.sin((x / 25) * Math.PI + phase) + amplitude * 0.5 * Math.sin((x / 16.7) * Math.PI - phase * 1.3);
        points.push(`${x ? 'L' : 'M'}${x} ${y.toFixed(2)}`);
    }
    return points.join(' ');
}

function mark(id: string, mold: string, strokeWidth: number): string {
    const surface = moltenSurface();
    return [
        `<clipPath id="${id}"><path d="${T_PATH}"/></clipPath>`,
        `<g clip-path="url(#${id})"><path fill="${palette.molten}" d="${surface} L100 100 L0 100 Z"/><path fill="none" stroke="${palette.ember}" stroke-width="${strokeWidth * 0.8}" d="${surface}"/></g>`,
        `<path fill="none" stroke="${mold}" stroke-width="${strokeWidth}" stroke-linejoin="round" d="${T_PATH}"/>`,
    ].join('');
}

const font = opentype.parse((await readFile(fontPath)).buffer);
const SIZE = 100;
const BASELINE = 0;

const round = (value: number) => +value.toFixed(2);

// opentype.js's own toPathData() emits `NaN` for some of Zilla Slab's points, so serialize the commands directly.
function pathData({ commands }: Path): string {
    return commands
        .map(c => {
            if (c.type === 'Z') return 'Z';
            if (c.type === 'Q') return `Q${round(c.x1)} ${round(c.y1)} ${round(c.x)} ${round(c.y)}`;
            if (c.type === 'C') return `C${round(c.x1)} ${round(c.y1)} ${round(c.x2)} ${round(c.y2)} ${round(c.x)} ${round(c.y)}`;
            return `${c.type}${round(c.x)} ${round(c.y)}`;
        })
        .join('');
}

/** Matches the landing sketch: the T is .95em wide and pulls in .075em on the left and .06em on the right. */
function wordmark(ink: string, mold: string): string {
    const fon = font.getPath('fon', 0, BASELINE, SIZE, { kerning: true });
    const tX = font.getAdvanceWidth('fon', SIZE, { kerning: true }) - 0.075 * SIZE;
    const tScale = (0.95 * SIZE) / T_BOX.width;
    const eX = tX + 0.95 * SIZE - 0.06 * SIZE;
    const e = font.getPath('e', eX, BASELINE, SIZE);

    const top = Math.min(fon.getBoundingBox().y1, BASELINE - T_BOX.height * tScale);
    const right = Math.max(e.getBoundingBox().x2, tX + T_BOX.width * tScale);
    const pad = 2;
    const viewBox = [-pad, top - pad, right + 2 * pad, BASELINE - top + 2 * pad].map(n => +n.toFixed(2)).join(' ');
    const ty = BASELINE - T_BOX.height * tScale;

    return [
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}" role="img" aria-label="fonte">`,
        `<path fill="${ink}" d="${pathData(fon)} ${pathData(e)}"/>`,
        `<g transform="translate(${tX.toFixed(2)} ${ty.toFixed(2)}) scale(${tScale})">${mark('t', mold, 2.4)}</g>`,
        '</svg>\n',
    ].join('');
}

// The favicon follows the viewer's color scheme, since browser tabs can be light or dark.
const logo = [
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="8 12 84 80" role="img" aria-label="fonte">',
    `<style>.mold{stroke:${palette.mold.light}}@media (prefers-color-scheme:dark){.mold{stroke:${palette.mold.dark}}}</style>`,
    mark('t', palette.mold.light, 3.2).replace('<path fill="none" stroke="', '<path class="mold" fill="none" stroke="'),
    '</svg>\n',
].join('');

const files = {
    'logo.svg': logo,
    'wordmark-light.svg': wordmark(palette.iron, palette.mold.light),
    'wordmark-dark.svg': wordmark(palette.ash, palette.mold.dark),
};
await Promise.all(
    Object.entries(files).map(async ([name, svg]) => {
        await writeFile(resolve(outDir, name), svg);
        console.log(`Generated ${resolve(outDir, name)}`);
    }),
);
