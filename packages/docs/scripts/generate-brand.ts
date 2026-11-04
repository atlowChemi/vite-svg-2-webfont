import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import type { Path } from 'opentype.js';

/**
 * Brand assets:
 * - `assets/svg/logo.svg` (copied to `public/` by `optimize-svg`): the T mark, used as the favicon;
 * - `assets/svg/social-card.svg` and `github-social-preview.svg`: the link preview and the repository's social
 *   preview, rendered to PNG by `generate-social-card.ts`;
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

/** The tools' filled 24×24 marks for the page cards: Simple Icons 13.0.0, CC0 1.0, https://simpleicons.org. */
const toolMarks = {
    rust: 'M23.8346 11.7033l-1.0073-.6236a13.7268 13.7268 0 00-.0283-.2936l.8656-.8069a.3483.3483 0 00-.1154-.578l-1.1066-.414a8.4958 8.4958 0 00-.087-.2856l.6904-.9587a.3462.3462 0 00-.2257-.5446l-1.1663-.1894a9.3574 9.3574 0 00-.1407-.2622l.49-1.0761a.3437.3437 0 00-.0274-.3361.3486.3486 0 00-.3006-.154l-1.1845.0416a6.7444 6.7444 0 00-.1873-.2268l.2723-1.153a.3472.3472 0 00-.417-.4172l-1.1532.2724a14.0183 14.0183 0 00-.2278-.1873l.0415-1.1845a.3442.3442 0 00-.49-.328l-1.076.491c-.0872-.0476-.1742-.0952-.2623-.1407l-.1903-1.1673A.3483.3483 0 0016.256.955l-.9597.6905a8.4867 8.4867 0 00-.2855-.086l-.414-1.1066a.3483.3483 0 00-.5781-.1154l-.8069.8666a9.2936 9.2936 0 00-.2936-.0284L12.2946.1683a.3462.3462 0 00-.5892 0l-.6236 1.0073a13.7383 13.7383 0 00-.2936.0284L9.9803.3374a.3462.3462 0 00-.578.1154l-.4141 1.1065c-.0962.0274-.1903.0567-.2855.086L7.744.955a.3483.3483 0 00-.5447.2258L7.009 2.348a9.3574 9.3574 0 00-.2622.1407l-1.0762-.491a.3462.3462 0 00-.49.328l.0416 1.1845a7.9826 7.9826 0 00-.2278.1873L3.8413 3.425a.3472.3472 0 00-.4171.4171l.2713 1.1531c-.0628.075-.1255.1509-.1863.2268l-1.1845-.0415a.3462.3462 0 00-.328.49l.491 1.0761a9.167 9.167 0 00-.1407.2622l-1.1662.1894a.3483.3483 0 00-.2258.5446l.6904.9587a13.303 13.303 0 00-.087.2855l-1.1065.414a.3483.3483 0 00-.1155.5781l.8656.807a9.2936 9.2936 0 00-.0283.2935l-1.0073.6236a.3442.3442 0 000 .5892l1.0073.6236c.008.0982.0182.1964.0283.2936l-.8656.8079a.3462.3462 0 00.1155.578l1.1065.4141c.0273.0962.0567.1914.087.2855l-.6904.9587a.3452.3452 0 00.2268.5447l1.1662.1893c.0456.088.0922.1751.1408.2622l-.491 1.0762a.3462.3462 0 00.328.49l1.1834-.0415c.0618.0769.1235.1528.1873.2277l-.2713 1.1541a.3462.3462 0 00.4171.4161l1.153-.2713c.075.0638.151.1255.2279.1863l-.0415 1.1845a.3442.3442 0 00.49.327l1.0761-.49c.087.0486.1741.0951.2622.1407l.1903 1.1662a.3483.3483 0 00.5447.2268l.9587-.6904a9.299 9.299 0 00.2855.087l.414 1.1066a.3452.3452 0 00.5781.1154l.8079-.8656c.0972.0111.1954.0203.2936.0294l.6236 1.0073a.3472.3472 0 00.5892 0l.6236-1.0073c.0982-.0091.1964-.0183.2936-.0294l.8069.8656a.3483.3483 0 00.578-.1154l.4141-1.1066a8.4626 8.4626 0 00.2855-.087l.9587.6904a.3452.3452 0 00.5447-.2268l.1903-1.1662c.088-.0456.1751-.0931.2622-.1407l1.0762.49a.3472.3472 0 00.49-.327l-.0415-1.1845a6.7267 6.7267 0 00.2267-.1863l1.1531.2713a.3472.3472 0 00.4171-.416l-.2713-1.1542c.0628-.0749.1255-.1508.1863-.2278l1.1845.0415a.3442.3442 0 00.328-.49l-.49-1.076c.0475-.0872.0951-.1742.1407-.2623l1.1662-.1893a.3483.3483 0 00.2258-.5447l-.6904-.9587.087-.2855 1.1066-.414a.3462.3462 0 00.1154-.5781l-.8656-.8079c.0101-.0972.0202-.1954.0283-.2936l1.0073-.6236a.3442.3442 0 000-.5892zm-6.7413 8.3551a.7138.7138 0 01.2986-1.396.714.714 0 11-.2997 1.396zm-.3422-2.3142a.649.649 0 00-.7715.5l-.3573 1.6685c-1.1035.501-2.3285.7795-3.6193.7795a8.7368 8.7368 0 01-3.6951-.814l-.3574-1.6684a.648.648 0 00-.7714-.499l-1.473.3158a8.7216 8.7216 0 01-.7613-.898h7.1676c.081 0 .1356-.0141.1356-.088v-2.536c0-.074-.0536-.0881-.1356-.0881h-2.0966v-1.6077h2.2677c.2065 0 1.1065.0587 1.394 1.2088.0901.3533.2875 1.5044.4232 1.8729.1346.413.6833 1.2381 1.2685 1.2381h3.5716a.7492.7492 0 00.1296-.0131 8.7874 8.7874 0 01-.8119.9526zM6.8369 20.024a.714.714 0 11-.2997-1.396.714.714 0 01.2997 1.396zM4.1177 8.9972a.7137.7137 0 11-1.304.5791.7137.7137 0 011.304-.579zm-.8352 1.9813l1.5347-.6824a.65.65 0 00.33-.8585l-.3158-.7147h1.2432v5.6025H3.5669a8.7753 8.7753 0 01-.2834-3.348zm6.7343-.5437V8.7836h2.9601c.153 0 1.0792.1772 1.0792.8697 0 .575-.7107.7815-1.2948.7815zm10.7574 1.4862c0 .2187-.008.4363-.0243.651h-.9c-.09 0-.1265.0586-.1265.1477v.413c0 .973-.5487 1.1846-1.0296 1.2382-.4576.0517-.9648-.1913-1.0275-.4717-.2704-1.5186-.7198-1.8436-1.4305-2.4034.8817-.5599 1.799-1.386 1.799-2.4915 0-1.1936-.819-1.9458-1.3769-2.3153-.7825-.5163-1.6491-.6195-1.883-.6195H5.4682a8.7651 8.7651 0 014.907-2.7699l1.0974 1.151a.648.648 0 00.9182.0213l1.227-1.1743a8.7753 8.7753 0 016.0044 4.2762l-.8403 1.8982a.652.652 0 00.33.8585l1.6178.7188c.0283.2875.0425.577.0425.8717zm-9.3006-9.5993a.7128.7128 0 11.984 1.0316.7137.7137 0 01-.984-1.0316zm8.3389 6.71a.7107.7107 0 01.9395-.3625.7137.7137 0 11-.9405.3635z',
    node: 'M11.998,24c-0.321,0-0.641-0.084-0.922-0.247l-2.936-1.737c-0.438-0.245-0.224-0.332-0.08-0.383 c0.585-0.203,0.703-0.25,1.328-0.604c0.065-0.037,0.151-0.023,0.218,0.017l2.256,1.339c0.082,0.045,0.197,0.045,0.272,0l8.795-5.076 c0.082-0.047,0.134-0.141,0.134-0.238V6.921c0-0.099-0.053-0.192-0.137-0.242l-8.791-5.072c-0.081-0.047-0.189-0.047-0.271,0 L3.075,6.68C2.99,6.729,2.936,6.825,2.936,6.921v10.15c0,0.097,0.054,0.189,0.139,0.235l2.409,1.392 c1.307,0.654,2.108-0.116,2.108-0.89V7.787c0-0.142,0.114-0.253,0.256-0.253h1.115c0.139,0,0.255,0.112,0.255,0.253v10.021 c0,1.745-0.95,2.745-2.604,2.745c-0.508,0-0.909,0-2.026-0.551L2.28,18.675c-0.57-0.329-0.922-0.945-0.922-1.604V6.921 c0-0.659,0.353-1.275,0.922-1.603l8.795-5.082c0.557-0.315,1.296-0.315,1.848,0l8.794,5.082c0.57,0.329,0.924,0.944,0.924,1.603 v10.15c0,0.659-0.354,1.273-0.924,1.604l-8.794,5.078C12.643,23.916,12.324,24,11.998,24z M19.099,13.993 c0-1.9-1.284-2.406-3.987-2.763c-2.731-0.361-3.009-0.548-3.009-1.187c0-0.528,0.235-1.233,2.258-1.233 c1.807,0,2.473,0.389,2.747,1.607c0.024,0.115,0.129,0.199,0.247,0.199h1.141c0.071,0,0.138-0.031,0.186-0.081 c0.048-0.054,0.074-0.123,0.067-0.196c-0.177-2.098-1.571-3.076-4.388-3.076c-2.508,0-4.004,1.058-4.004,2.833 c0,1.925,1.488,2.457,3.895,2.695c2.88,0.282,3.103,0.703,3.103,1.269c0,0.983-0.789,1.402-2.642,1.402 c-2.327,0-2.839-0.584-3.011-1.742c-0.02-0.124-0.126-0.215-0.253-0.215h-1.137c-0.141,0-0.254,0.112-0.254,0.253 c0,1.482,0.806,3.248,4.655,3.248C17.501,17.007,19.099,15.91,19.099,13.993z',
    wasm: 'M14.745,0c0,0.042,0,0.085,0,0.129c0,1.52-1.232,2.752-2.752,2.752c-1.52,0-2.752-1.232-2.752-2.752 c0-0.045,0-0.087,0-0.129H0v24h24V0H14.745z M11.454,21.431l-1.169-5.783h-0.02l-1.264,5.783H7.39l-1.824-8.497h1.59l1.088,5.783 h0.02l1.311-5.783h1.487l1.177,5.854h0.02l1.242-5.854h1.561l-2.027,8.497H11.454z M20.209,21.431l-0.542-1.891h-2.861l-0.417,1.891 h-1.59l2.056-8.497h2.509l2.5,8.497H20.209z M17.812,15.028l-0.694,3.118h2.159l-0.796-3.118H17.812z',
    vite: 'm8.286 10.578.512-8.657a.306.306 0 0 1 .247-.282L17.377.006a.306.306 0 0 1 .353.385l-1.558 5.403a.306.306 0 0 0 .352.385l2.388-.46a.306.306 0 0 1 .332.438l-6.79 13.55-.123.19a.294.294 0 0 1-.252.14c-.177 0-.35-.152-.305-.369l1.095-5.301a.306.306 0 0 0-.388-.355l-1.433.435a.306.306 0 0 1-.389-.354l.69-3.375a.306.306 0 0 0-.37-.36l-2.32.536a.306.306 0 0 1-.374-.316zm14.976-7.926L17.284 3.74l-.544 1.887 2.077-.4a.8.8 0 0 1 .84.369.8.8 0 0 1 .034.783L12.9 19.93l-.013.025-.015.023-.122.19a.801.801 0 0 1-.672.37.826.826 0 0 1-.634-.302.8.8 0 0 1-.16-.67l1.029-4.981-1.12.34a.81.81 0 0 1-.86-.262.802.802 0 0 1-.165-.67l.63-3.08-2.027.468a.808.808 0 0 1-.768-.233.81.81 0 0 1-.217-.6l.389-6.57-7.44-1.33a.612.612 0 0 0-.64.906L11.58 23.691a.612.612 0 0 0 1.066-.004l11.26-20.135a.612.612 0 0 0-.644-.9z',
} as const;
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

/** Matches the landing page: the T is .95em wide and pulls in .075em on the left and .06em on the right. */
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

const mono = await loadFont('@fontsource/jetbrains-mono/files/jetbrains-mono-latin-500-normal.woff');
const iron = '#1c1a1f';

/** A line of text as outlines. */
const text = (font: typeof zilla, value: string, x: number, y: number, size: number, fill: string) =>
    `<path fill="${fill}" d="${pathData(font.getPath(value, x, y, size, { kerning: true }))}"/>`;

/**
 * The social cards: the tagline beside one large molten T, on iron. Text is outlined like the wordmark's letters, so
 * `generate-social-card.ts` renders them without loading fonts.
 */
function socialCard(title: string, width: number, height: number, margin: number): string {
    // The T fills the card's height, leaving room for the molten bar along the bottom edge.
    const bar = 24;
    const tScale = (height - bar - 2 * margin) / (T_BOX.height - 18);
    const tLeft = width - margin - 86 * tScale;
    const tTop = (height - bar - (T_BOX.height - 18) * tScale) / 2 - 18 * tScale;

    const wordmarkHeight = 84;
    const wordmarkScale = wordmarkHeight / (BASELINE - layout.top);
    const wordmark = `<g transform="translate(${margin} ${round(margin + wordmarkHeight)}) scale(${round(wordmarkScale)})"><path fill="${palette.ink.dark}" d="${layout.letters}"/><g transform="${layout.tTransform}">${mark('wordmark', { mold: { color: palette.mold.dark }, strokeWidth: 2.4 })}</g></g>`;

    const taglineSize = 78;
    const lines = ['Melted SVGs,', 'cast into fonts'];
    const widest = Math.max(...lines.map(line => zilla.getAdvanceWidth(line, taglineSize, { kerning: true })));
    if (margin + widest > tLeft + 14 * tScale - 24) throw new Error(`The ${title} tagline runs into the T.`);
    const baseline = height / 2 + 15;

    return [
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title">`,
        `<title id="title">${title}</title>`,
        `<path fill="${iron}" d="M0 0H${width}V${height}H0Z"/>`,
        `<g transform="translate(${round(tLeft)} ${round(tTop)}) scale(${round(tScale)})">${mark('card', { mold: { color: palette.mold.dark }, strokeWidth: 0.9 })}</g>`,
        wordmark,
        ...lines.map((line, i) => text(zilla, line, margin, baseline + i * 88, taglineSize, palette.ink.dark)),
        text(mono, 'Vite · Node.js · Browser · CLI · Rust', margin, height - bar - margin + 8, 23, palette.mold.dark),
        `<path fill="${palette.molten}" d="M0 ${height - bar}H${width}V${height}H0Z"/>`,
        '</svg>\n',
    ].join('');
}

type ToolMark = keyof typeof toolMarks;

/**
 * A doc page's social card: the page title on ash, with its tool's plate and mark, as on the page itself. Not wired
 * into the build yet; `assets/social/page-card.svg` is a sample of the template.
 */
function pageCard({ plate, title, detail, url, tool }: { plate: string; title: string; detail: string; url: string; tool: ToolMark }): string {
    const [width, height, margin] = [1200, 630, 72];
    const ink = palette.ink.light;
    const muted = palette.mold.light;
    const markSize = 168;

    // Wrap the title in Zilla Slab to the space left of the mark, two lines at most.
    const titleSize = 84;
    const maxWidth = width - 2 * margin - markSize - 48;
    const lines: string[] = [];
    for (const word of title.split(' ')) {
        const last = lines.at(-1);
        if (last && zilla.getAdvanceWidth(`${last} ${word}`, titleSize, { kerning: true }) <= maxWidth) lines[lines.length - 1] = `${last} ${word}`;
        else lines.push(word);
    }
    if (lines.length > 2) throw new Error(`The page card title "${title}" needs more than two lines.`);

    const wordmarkHeight = 50;
    const wordmarkScale = wordmarkHeight / (BASELINE - layout.top);
    const rule = (y: number) => `<path stroke="${ink}" stroke-opacity=".18" stroke-width="2" d="M${margin} ${y}H${width - margin}"/>`;
    const plateWidth = mono.getAdvanceWidth(plate, 22);

    return [
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title">`,
        `<title id="title">${title}, ${plate}: fonte</title>`,
        `<path fill="#faf7f2" d="M0 0H${width}V${height}H0Z"/>`,
        `<g transform="translate(${margin} ${round(56 + wordmarkHeight)}) scale(${round(wordmarkScale)})"><path fill="${ink}" d="${layout.letters}"/><g transform="${layout.tTransform}">${mark('page-wordmark', { mold: { color: muted }, strokeWidth: 2.4 })}</g></g>`,
        text(mono, plate, round(width - margin - plateWidth), 94, 22, '#b4410a'),
        rule(128),
        ...lines.map((line, i) => text(zilla, line, margin, (lines.length === 1 ? 320 : 276) + i * 96, titleSize, ink)),
        text(mono, detail, margin, 452, 24, muted),
        `<g transform="translate(${width - margin - markSize} 360) scale(${markSize / 24})"><path fill="${palette.molten}" d="${toolMarks[tool]}"/></g>`,
        rule(556),
        text(mono, url, margin, 596, 22, muted),
        '</svg>\n',
    ].join('');
}

const brandDir = resolve(docsDir, 'assets', 'brand');
const files = {
    [resolve(docsDir, 'assets', 'svg', 'logo.svg')]: logo,
    // GitHub's social preview is 1280×640 and asks for a 40pt safe border.
    [resolve(docsDir, 'assets', 'svg', 'social-card.svg')]: socialCard('fonte: melted SVGs, cast into fonts', 1200, 630, 72),
    [resolve(docsDir, 'assets', 'svg', 'github-social-preview.svg')]: socialCard('fonte: melted SVGs, cast into fonts', 1280, 640, 88),
    [resolve(docsDir, 'assets', 'social', 'page-card.svg')]: pageCard({
        plate: 'npm · Node.js',
        title: 'Configuration',
        detail: '@atlowchemi/webfont-generator',
        url: 'fonte.atlow.dev/node/configuration',
        tool: 'node',
    }),
    [resolve(docsDir, '.vitepress', 'theme', 'wordmark.ts')]: component,
    [resolve(brandDir, 'fonte.svg')]: animatedWordmark,
    [resolve(brandDir, 'vite.svg')]: viteEmblem(),
    [resolve(brandDir, 'node.svg')]: nodeEmblem(),
    [resolve(brandDir, 'browser.svg')]: browserEmblem(),
    [resolve(brandDir, 'rust.svg')]: rustEmblem(),
};
await mkdir(brandDir, { recursive: true });
await mkdir(resolve(docsDir, 'assets', 'social'), { recursive: true });
await Promise.all(
    Object.entries(files).map(async ([path, content]) => {
        await writeFile(path, content);
        console.log(`Generated ${path}`);
    }),
);
