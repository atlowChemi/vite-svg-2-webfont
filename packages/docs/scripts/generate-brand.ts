import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import type { Path } from 'opentype.js';

/**
 * Brand assets:
 * - `assets/svg/logo.svg` (copied to `public/` by `optimize-svg`): the T mark, used as the favicon;
 * - `.vitepress/theme/wordmark.ts`: the `fonTe` wordmark's shapes for the in-page `FonteWordmark` component, which
 *   colors them from the site theme;
 * - `assets/brand/`: the READMEs' logos, loaded from the repository rather than the site. `fonte.svg` is the wordmark
 *   with the landing page's pour and waves; each package's emblem casts the T in a mold shaped like its tool's emblem.
 *
 * The wordmark's `fon` and `e` are Zilla Slab SemiBold outlines, so they
 * render identically without the web fonts (READMEs show SVGs as images, which can't load fonts).
 * Re-run after changing the palette or the mark: `node ./scripts/generate-brand.ts`.
 */
const docsDir = resolve(import.meta.dirname, '..');
const require = createRequire(import.meta.url);
// opentype.js is CommonJS without named-export hints, so Node's ESM loader only sees a default export.
const { parse } = require('opentype.js') as typeof import('opentype.js');
const loadFont = async (file: string) => parse((await readFile(require.resolve(file))).buffer);

const palette = {
    molten: '#ff6a13',
    ember: '#ffb02e',
    mold: { light: '#5a5258', dark: '#b3abba' },
    ink: { light: '#1c1a1f', dark: '#f3ece2' },
};

/** The bracketed-serif T in a 100×86 box; its bottom edge sits on the text baseline. */
const T_PATH = 'M14 18 H86 V42 Q84 34 76 32 H57 V78 Q57 82 62 82 H68 V86 H32 V82 H38 Q43 82 43 78 V32 H24 Q16 34 14 42 Z';
const T_BOX = { width: 100, height: 86 };
/** The metal level in the T's box: empty at the stem's foot, full at the crossbar. */
const LEVEL = { empty: 90, full: 22 };

/** A still frame of the landing page's molten surface, at the full-mold level. */
function moltenSurface(level = LEVEL.full, amplitude = 1.3, phase = 0): string {
    const points: string[] = [];
    for (let x = 0; x <= 100; x += 4) {
        const y = level + amplitude * Math.sin((x / 25) * Math.PI + phase) + amplitude * 0.5 * Math.sin((x / 16.7) * Math.PI - phase * 1.3);
        points.push(`${x ? 'L' : 'M'}${x} ${y.toFixed(2)}`);
    }
    return points.join(' ');
}

/**
 * The landing page's surface reshaped to repeat every 50 units, so sliding it one period loops seamlessly. It spans
 * one period either side of the T's box, which is what the drift and the clip need.
 */
function periodicSurface(level = LEVEL.full, amplitude = 1.3): string {
    const points: string[] = [];
    for (let x = -50; x <= 150; x += 2) {
        const y = level + amplitude * Math.sin((x / 25) * Math.PI) + amplitude * 0.5 * Math.sin((x / 12.5) * Math.PI + 1);
        points.push(`${x === -50 ? 'M' : 'L'}${x} ${y.toFixed(2)}`);
    }
    return points.join(' ');
}

/** The T: metal clipped to the mold, its surface line, and the mold outline. `mold` is a color or, for themed SVGs, a class. */
function mark(id: string, { mold, strokeWidth, surface = moltenSurface() }: { mold: { color: string } | { className: string }; strokeWidth: number; surface?: string }) {
    const moldPaint = 'color' in mold ? `stroke="${mold.color}"` : `class="${mold.className}"`;
    return [
        `<clipPath id="${id}"><path d="${T_PATH}"/></clipPath>`,
        `<g clip-path="url(#${id})">${metal(surface, strokeWidth)}</g>`,
        `<path fill="none" ${moldPaint} stroke-width="${strokeWidth}" stroke-linejoin="round" d="${T_PATH}"/>`,
    ].join('');
}

function metal(surface: string, strokeWidth: number): string {
    return `<path fill="${palette.molten}" d="${surface} L150 100 L-50 100 Z"/><path fill="none" stroke="${palette.ember}" stroke-width="${round(strokeWidth * 0.8)}" d="${surface}"/>`;
}

const zilla = await loadFont('@fontsource/zilla-slab/files/zilla-slab-latin-600-normal.woff');
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
function layoutWordmark() {
    const fon = zilla.getPath('fon', 0, BASELINE, SIZE, { kerning: true });
    const tX = zilla.getAdvanceWidth('fon', SIZE, { kerning: true }) - 0.075 * SIZE;
    const tScale = (0.95 * SIZE) / T_BOX.width;
    const eX = tX + 0.95 * SIZE - 0.06 * SIZE;
    const e = zilla.getPath('e', eX, BASELINE, SIZE);

    const top = Math.min(fon.getBoundingBox().y1, BASELINE - T_BOX.height * tScale);
    const right = Math.max(e.getBoundingBox().x2, tX + T_BOX.width * tScale);
    const pad = 2;
    return {
        top,
        right,
        viewBox: [-pad, top - pad, right + 2 * pad, BASELINE - top + 2 * pad].map(round).join(' '),
        letters: `${pathData(fon)} ${pathData(e)}`,
        tTransform: `translate(${round(tX)} ${round(BASELINE - T_BOX.height * tScale)}) scale(${tScale})`,
    };
}

const layout = layoutWordmark();

// Lines past the repository's 180-column print width break after `=`, exactly as `vp fmt` would write them.
const exportString = (name: string, value: string) => {
    const line = `export const ${name} = '${value}';`;
    return line.length > 180 ? `export const ${name} =\n    '${value}';` : line;
};
const component = [
    '// Generated by scripts/generate-brand.ts (`vp run @atlowchemi/vite-svg-webfont-docs#brand`); do not edit.',
    exportString('viewBox', layout.viewBox),
    exportString('letters', layout.letters),
    exportString('tTransform', layout.tTransform),
    exportString('tPath', T_PATH),
    exportString('moltenSurface', moltenSurface()),
    '',
].join('\n');

// The favicon and README logos follow the viewer's color scheme, since browser tabs and READMEs can be light or dark.
// Classes: `.mold` strokes in the mold's color, `.ink` fills in the text color.
const themed = (classes: ('mold' | 'ink')[], extra = '') => {
    const rule = { mold: ['stroke', 'mold'], ink: ['fill', 'ink'] } as const;
    const css = (scheme: 'light' | 'dark') =>
        classes
            .map(name => {
                const [property, color] = rule[name];
                return `.${name}{${property}:${palette[color][scheme]}}`;
            })
            .join('');
    return `<style>${css('light')}@media (prefers-color-scheme:dark){${css('dark')}}${extra}</style>`;
};

const logo = [
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="8 12 84 80" role="img" aria-label="fonte">',
    themed(['mold']),
    mark('t', { mold: { className: 'mold' }, strokeWidth: 3.2 }),
    '</svg>\n',
].join('');

const wordmarkBody = (t: string) => `<path class="ink" d="${layout.letters}"/><g transform="${layout.tTransform}">${t}</g>`;

// The root README's wordmark: the metal pours into the empty T once, then its surface drifts like the landing page's
// waves. Both are CSS animations, so viewers who prefer reduced motion get the full, still mold.
const pourDepth = LEVEL.empty - LEVEL.full;
const animatedWordmark = [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${layout.viewBox}" role="img" aria-label="fonte">`,
    themed(
        ['ink', 'mold'],
        [
            `.pour{animation:pour 2.4s cubic-bezier(.25,.7,.3,1) both}`,
            `.drift{animation:drift 3.2s linear infinite}`,
            `@keyframes pour{from{transform:translateY(${pourDepth}px)}}`,
            `@keyframes drift{to{transform:translateX(-50px)}}`,
            `@media (prefers-reduced-motion:reduce){.pour,.drift{animation:none}}`,
        ].join(''),
    ),
    wordmarkBody(
        [
            `<clipPath id="t"><path d="${T_PATH}"/></clipPath>`,
            `<g clip-path="url(#t)"><g class="pour"><g class="drift">${metal(periodicSurface(), 1.9)}</g></g></g>`,
            `<path fill="none" class="mold" stroke-width="2.4" stroke-linejoin="round" d="${T_PATH}"/>`,
        ].join(''),
    ),
    '</svg>\n',
].join('');

/**
 * The packages' emblems: the T cast in a mold shaped like its tool's own emblem, on a dark casting floor, so they read
 * on light and dark READMEs alike. Each is a 512×512 badge.
 */
const EMBLEM = 512;
// Where the T's 100×86 box lands, and how large: centred on the mold's visible bounds (x 14–86, y 18–86).
const placeT = (cx: number, cy: number, scale: number) => `translate(${round(cx - 50 * scale)} ${round(cy - 52 * scale)}) scale(${scale})`;

/** Shared defs: the metal's heat gradient, its glow, and the mold's iron. */
const emblemDefs = [
    `<linearGradient id="heat" x1="0" y1="20" x2="0" y2="86" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#ffd27a"/><stop offset=".12" stop-color="${palette.ember}"/><stop offset=".45" stop-color="${palette.molten}"/><stop offset="1" stop-color="#b8360a"/></linearGradient>`,
    `<linearGradient id="iron" x1="0" y1="18" x2="0" y2="86" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#d9d2dd"/><stop offset=".5" stop-color="${palette.mold.dark}"/><stop offset="1" stop-color="#6f6774"/></linearGradient>`,
    `<filter id="glow" x="-40%" y="-40%" width="180%" height="180%"><feGaussianBlur stdDeviation="5" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>`,
].join('');

/** The molten T: metal clipped to the mold, glowing past it, under the iron outline. */
function castT(transform: string, surface = moltenSurface()): string {
    return [
        `<g transform="${transform}">`,
        `<clipPath id="mold"><path d="${T_PATH}"/></clipPath>`,
        `<g filter="url(#glow)"><g clip-path="url(#mold)"><path fill="url(#heat)" d="${surface} L150 100 L-50 100 Z"/><path fill="none" stroke="#ffe3a3" stroke-width="1.4" d="${surface}"/></g></g>`,
        `<path fill="none" stroke="url(#iron)" stroke-width="2.6" stroke-linejoin="round" d="${T_PATH}"/>`,
        '</g>',
    ].join('');
}

const emblemSvg = (label: string, defs: string, body: string) =>
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${EMBLEM} ${EMBLEM}" role="img" aria-label="${label}"><defs>${emblemDefs}${defs}</defs>${body}</svg>\n`;

/** Node.js: the T cast in Node's hexagon, rimmed in its greens. */
function nodeEmblem(): string {
    const hex = 'M256 28 453 142V370L256 484 59 370V142Z';
    const inner = 'M256 54 431 155V357L256 458 81 357V155Z';
    return emblemSvg(
        'fonte for Node.js',
        '<linearGradient id="rim" x1="110" y1="40" x2="400" y2="470" gradientUnits="userSpaceOnUse"><stop stop-color="#a3e879"/><stop offset=".5" stop-color="#539e43"/><stop offset="1" stop-color="#246b39"/></linearGradient>',
        [
            `<path d="${hex}" fill="#10231a" stroke="url(#rim)" stroke-width="24" stroke-linejoin="round"/>`,
            `<path d="${inner}" fill="none" stroke="#b5f399" stroke-opacity=".22" stroke-width="2"/>`,
            castT(placeT(256, 262, 3)),
        ].join(''),
    );
}

/** WebAssembly's rounded square with a semicircular bite out of the top edge, inset by `inset`. */
function wasmPlate(inset: number): string {
    const [l, t, r, b, radius] = [44 + inset, 44 + inset, 468 - inset, 468 - inset, 40 - inset / 2];
    const notch = 58 + inset;
    return `M${l + radius} ${t}H${256 - notch}A${notch} ${notch} 0 0 0 ${256 + notch} ${t}H${r - radius}Q${r} ${t} ${r} ${t + radius}V${b - radius}Q${r} ${b} ${r - radius} ${b}H${l + radius}Q${l} ${b} ${l} ${b - radius}V${t + radius}Q${l} ${t} ${l + radius} ${t}Z`;
}

/** The browser: WebAssembly's notched square, its notch the sprue that pours the metal into the T. */
function browserEmblem(): string {
    // The stream narrows as it falls, from the sprue to the metal's surface.
    const top = 44 + 22 + 40;
    const [cx, cy, scale] = [256, 300, 2.75];
    const surfaceY = cy + (LEVEL.full - 52) * scale;
    const stream = `M${256 - 13} ${top}C${256 - 10} ${top + 30} ${256 - 6} ${surfaceY - 30} ${256 - 6} ${surfaceY}H${256 + 6}C${256 + 6} ${surfaceY - 30} ${256 + 10} ${top + 30} ${256 + 13} ${top}Z`;
    return emblemSvg(
        'fonte for the browser',
        [
            '<linearGradient id="wasm" x1="60" y1="40" x2="440" y2="480" gradientUnits="userSpaceOnUse"><stop stop-color="#8f80ff"/><stop offset=".55" stop-color="#654ff0"/><stop offset="1" stop-color="#3f2db5"/></linearGradient>',
            `<linearGradient id="pour" x1="0" y1="${top}" x2="0" y2="${surfaceY}" gradientUnits="userSpaceOnUse"><stop stop-color="#ffe3a3"/><stop offset=".4" stop-color="${palette.ember}"/><stop offset="1" stop-color="${palette.molten}"/></linearGradient>`,
        ].join(''),
        [
            `<path d="${wasmPlate(0)}" fill="url(#wasm)"/>`,
            `<path d="${wasmPlate(22)}" fill="#141030"/>`,
            castT(placeT(cx, cy, scale), moltenSurface(LEVEL.full, 2.2, 0.9)),
            `<g filter="url(#glow)"><path d="${stream}" fill="url(#pour)"/><ellipse cx="256" cy="${round(surfaceY + 1)}" rx="30" ry="4.5" fill="none" stroke="#ffe3a3" stroke-width="2.5"/><circle cx="226" cy="${round(surfaceY - 14)}" r="4" fill="${palette.ember}"/><circle cx="289" cy="${round(surfaceY - 20)}" r="3.2" fill="${palette.ember}"/><circle cx="302" cy="${round(surfaceY - 6)}" r="2.2" fill="${palette.ember}"/></g>`,
        ].join(''),
    );
}

/** Rust: the T cast inside a rusted cast-iron gear. */
function rustEmblem(): string {
    const center = 256;
    const [outer, root, hole] = [238, 204, 168];
    const teeth = 16;
    const points: string[] = [];
    for (let i = 0; i < teeth; i++) {
        const step = (2 * Math.PI) / teeth;
        const a = i * step - Math.PI / 2;
        // Each tooth: up the flank, across the tip, down the flank, then along the root to the next tooth.
        for (const [offset, radius] of [
            [-0.3, root],
            [-0.2, outer],
            [0.2, outer],
            [0.3, root],
        ] as const) {
            const angle = a + offset * step;
            points.push(`${round(center + radius * Math.cos(angle))} ${round(center + radius * Math.sin(angle))}`);
        }
    }
    const gear = `M${points.join('L')}Z`;
    return emblemSvg(
        'fonte for Rust',
        [
            '<radialGradient id="iron-rust" cx="200" cy="170" r="360" gradientUnits="userSpaceOnUse"><stop stop-color="#d98a4e"/><stop offset=".45" stop-color="#a24f24"/><stop offset="1" stop-color="#4f2410"/></radialGradient>',
            // Pitting: fractal noise keeps only its darkest values, as specks that darken the iron.
            '<filter id="pitting" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency=".85" numOctaves="2" seed="7"/><feColorMatrix values="0 0 0 0 .16  0 0 0 0 .06  0 0 0 0 .02  -7 0 0 0 3.6"/><feComposite in2="SourceGraphic" operator="in"/></filter>',
        ].join(''),
        [
            `<path d="${gear}" fill="url(#iron-rust)" stroke="#3a1a0b" stroke-width="3" stroke-linejoin="round"/>`,
            `<path d="${gear}" fill="#000" filter="url(#pitting)"/>`,
            `<circle cx="${center}" cy="${center}" r="${root - 14}" fill="none" stroke="#3a1a0b" stroke-opacity=".5" stroke-width="2"/>`,
            `<circle cx="${center}" cy="${center}" r="${hole}" fill="#1b1310" stroke="#2a150a" stroke-width="6"/>`,
            castT(placeT(256, 258, 2.45)),
        ].join(''),
    );
}

/** Vite: the T cast in Vite's chevron, struck by its bolt, as the old logo's lightning crossed its letter. */
function viteEmblem(): string {
    const chevron = 'M40 74Q34 62 47 60L256 40 465 60Q478 62 472 74L270 478Q256 500 242 478Z';
    const inner = 'M78 86 256 68 434 86 256 444Z';
    const bolt = 'M356 132 266 286H310L238 424 370 246H324L382 132Z';
    return emblemSvg(
        'fonte for Vite',
        [
            '<linearGradient id="vite" x1="60" y1="40" x2="420" y2="460" gradientUnits="userSpaceOnUse"><stop stop-color="#41d1ff"/><stop offset="1" stop-color="#bd34fe"/></linearGradient>',
            '<linearGradient id="bolt" x1="340" y1="132" x2="270" y2="424" gradientUnits="userSpaceOnUse"><stop stop-color="#ffea83"/><stop offset=".1" stop-color="#ffdd35"/><stop offset="1" stop-color="#ffa800"/></linearGradient>',
        ].join(''),
        [
            `<path d="${chevron}" fill="url(#vite)"/>`,
            `<path d="${inner}" fill="#15122b" stroke="#15122b" stroke-width="28" stroke-linejoin="round"/>`,
            castT(placeT(244, 205, 2.55)),
            `<path d="${bolt}" fill="url(#bolt)" stroke="#15122b" stroke-width="6" stroke-linejoin="round" filter="url(#glow)"/>`,
        ].join(''),
    );
}

const brandDir = resolve(docsDir, 'assets', 'brand');
const files = {
    [resolve(docsDir, 'assets', 'svg', 'logo.svg')]: logo,
    [resolve(docsDir, '.vitepress', 'theme', 'wordmark.ts')]: component,
    [resolve(brandDir, 'fonte.svg')]: animatedWordmark,
    [resolve(brandDir, 'vite.svg')]: viteEmblem(),
    [resolve(brandDir, 'node.svg')]: nodeEmblem(),
    [resolve(brandDir, 'browser.svg')]: browserEmblem(),
    [resolve(brandDir, 'rust.svg')]: rustEmblem(),
};
await mkdir(brandDir, { recursive: true });
await Promise.all(
    Object.entries(files).map(async ([path, content]) => {
        await writeFile(path, content);
        console.log(`Generated ${path}`);
    }),
);
