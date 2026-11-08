import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import type { Path } from 'opentype.js';

// The brand's shapes, and the text outlined in its fonts: shared by `scripts/generate-brand.ts`, which writes the logos
// and the site-wide social card, and the build, which draws each doc page's link-preview card.

const require = createRequire(import.meta.url);
// opentype.js is CommonJS without named-export hints, so Node's ESM loader only sees a default export.
const { parse } = require('opentype.js') as typeof import('opentype.js');

/** The tools' filled 24×24 marks for the page cards: Simple Icons 13.0.0, CC0 1.0, https://simpleicons.org, and a prompt. */
const toolMarks = {
    rust: 'M23.8346 11.7033l-1.0073-.6236a13.7268 13.7268 0 00-.0283-.2936l.8656-.8069a.3483.3483 0 00-.1154-.578l-1.1066-.414a8.4958 8.4958 0 00-.087-.2856l.6904-.9587a.3462.3462 0 00-.2257-.5446l-1.1663-.1894a9.3574 9.3574 0 00-.1407-.2622l.49-1.0761a.3437.3437 0 00-.0274-.3361.3486.3486 0 00-.3006-.154l-1.1845.0416a6.7444 6.7444 0 00-.1873-.2268l.2723-1.153a.3472.3472 0 00-.417-.4172l-1.1532.2724a14.0183 14.0183 0 00-.2278-.1873l.0415-1.1845a.3442.3442 0 00-.49-.328l-1.076.491c-.0872-.0476-.1742-.0952-.2623-.1407l-.1903-1.1673A.3483.3483 0 0016.256.955l-.9597.6905a8.4867 8.4867 0 00-.2855-.086l-.414-1.1066a.3483.3483 0 00-.5781-.1154l-.8069.8666a9.2936 9.2936 0 00-.2936-.0284L12.2946.1683a.3462.3462 0 00-.5892 0l-.6236 1.0073a13.7383 13.7383 0 00-.2936.0284L9.9803.3374a.3462.3462 0 00-.578.1154l-.4141 1.1065c-.0962.0274-.1903.0567-.2855.086L7.744.955a.3483.3483 0 00-.5447.2258L7.009 2.348a9.3574 9.3574 0 00-.2622.1407l-1.0762-.491a.3462.3462 0 00-.49.328l.0416 1.1845a7.9826 7.9826 0 00-.2278.1873L3.8413 3.425a.3472.3472 0 00-.4171.4171l.2713 1.1531c-.0628.075-.1255.1509-.1863.2268l-1.1845-.0415a.3462.3462 0 00-.328.49l.491 1.0761a9.167 9.167 0 00-.1407.2622l-1.1662.1894a.3483.3483 0 00-.2258.5446l.6904.9587a13.303 13.303 0 00-.087.2855l-1.1065.414a.3483.3483 0 00-.1155.5781l.8656.807a9.2936 9.2936 0 00-.0283.2935l-1.0073.6236a.3442.3442 0 000 .5892l1.0073.6236c.008.0982.0182.1964.0283.2936l-.8656.8079a.3462.3462 0 00.1155.578l1.1065.4141c.0273.0962.0567.1914.087.2855l-.6904.9587a.3452.3452 0 00.2268.5447l1.1662.1893c.0456.088.0922.1751.1408.2622l-.491 1.0762a.3462.3462 0 00.328.49l1.1834-.0415c.0618.0769.1235.1528.1873.2277l-.2713 1.1541a.3462.3462 0 00.4171.4161l1.153-.2713c.075.0638.151.1255.2279.1863l-.0415 1.1845a.3442.3442 0 00.49.327l1.0761-.49c.087.0486.1741.0951.2622.1407l.1903 1.1662a.3483.3483 0 00.5447.2268l.9587-.6904a9.299 9.299 0 00.2855.087l.414 1.1066a.3452.3452 0 00.5781.1154l.8079-.8656c.0972.0111.1954.0203.2936.0294l.6236 1.0073a.3472.3472 0 00.5892 0l.6236-1.0073c.0982-.0091.1964-.0183.2936-.0294l.8069.8656a.3483.3483 0 00.578-.1154l.4141-1.1066a8.4626 8.4626 0 00.2855-.087l.9587.6904a.3452.3452 0 00.5447-.2268l.1903-1.1662c.088-.0456.1751-.0931.2622-.1407l1.0762.49a.3472.3472 0 00.49-.327l-.0415-1.1845a6.7267 6.7267 0 00.2267-.1863l1.1531.2713a.3472.3472 0 00.4171-.416l-.2713-1.1542c.0628-.0749.1255-.1508.1863-.2278l1.1845.0415a.3442.3442 0 00.328-.49l-.49-1.076c.0475-.0872.0951-.1742.1407-.2623l1.1662-.1893a.3483.3483 0 00.2258-.5447l-.6904-.9587.087-.2855 1.1066-.414a.3462.3462 0 00.1154-.5781l-.8656-.8079c.0101-.0972.0202-.1954.0283-.2936l1.0073-.6236a.3442.3442 0 000-.5892zm-6.7413 8.3551a.7138.7138 0 01.2986-1.396.714.714 0 11-.2997 1.396zm-.3422-2.3142a.649.649 0 00-.7715.5l-.3573 1.6685c-1.1035.501-2.3285.7795-3.6193.7795a8.7368 8.7368 0 01-3.6951-.814l-.3574-1.6684a.648.648 0 00-.7714-.499l-1.473.3158a8.7216 8.7216 0 01-.7613-.898h7.1676c.081 0 .1356-.0141.1356-.088v-2.536c0-.074-.0536-.0881-.1356-.0881h-2.0966v-1.6077h2.2677c.2065 0 1.1065.0587 1.394 1.2088.0901.3533.2875 1.5044.4232 1.8729.1346.413.6833 1.2381 1.2685 1.2381h3.5716a.7492.7492 0 00.1296-.0131 8.7874 8.7874 0 01-.8119.9526zM6.8369 20.024a.714.714 0 11-.2997-1.396.714.714 0 01.2997 1.396zM4.1177 8.9972a.7137.7137 0 11-1.304.5791.7137.7137 0 011.304-.579zm-.8352 1.9813l1.5347-.6824a.65.65 0 00.33-.8585l-.3158-.7147h1.2432v5.6025H3.5669a8.7753 8.7753 0 01-.2834-3.348zm6.7343-.5437V8.7836h2.9601c.153 0 1.0792.1772 1.0792.8697 0 .575-.7107.7815-1.2948.7815zm10.7574 1.4862c0 .2187-.008.4363-.0243.651h-.9c-.09 0-.1265.0586-.1265.1477v.413c0 .973-.5487 1.1846-1.0296 1.2382-.4576.0517-.9648-.1913-1.0275-.4717-.2704-1.5186-.7198-1.8436-1.4305-2.4034.8817-.5599 1.799-1.386 1.799-2.4915 0-1.1936-.819-1.9458-1.3769-2.3153-.7825-.5163-1.6491-.6195-1.883-.6195H5.4682a8.7651 8.7651 0 014.907-2.7699l1.0974 1.151a.648.648 0 00.9182.0213l1.227-1.1743a8.7753 8.7753 0 016.0044 4.2762l-.8403 1.8982a.652.652 0 00.33.8585l1.6178.7188c.0283.2875.0425.577.0425.8717zm-9.3006-9.5993a.7128.7128 0 11.984 1.0316.7137.7137 0 01-.984-1.0316zm8.3389 6.71a.7107.7107 0 01.9395-.3625.7137.7137 0 11-.9405.3635z',
    node: 'M11.998,24c-0.321,0-0.641-0.084-0.922-0.247l-2.936-1.737c-0.438-0.245-0.224-0.332-0.08-0.383 c0.585-0.203,0.703-0.25,1.328-0.604c0.065-0.037,0.151-0.023,0.218,0.017l2.256,1.339c0.082,0.045,0.197,0.045,0.272,0l8.795-5.076 c0.082-0.047,0.134-0.141,0.134-0.238V6.921c0-0.099-0.053-0.192-0.137-0.242l-8.791-5.072c-0.081-0.047-0.189-0.047-0.271,0 L3.075,6.68C2.99,6.729,2.936,6.825,2.936,6.921v10.15c0,0.097,0.054,0.189,0.139,0.235l2.409,1.392 c1.307,0.654,2.108-0.116,2.108-0.89V7.787c0-0.142,0.114-0.253,0.256-0.253h1.115c0.139,0,0.255,0.112,0.255,0.253v10.021 c0,1.745-0.95,2.745-2.604,2.745c-0.508,0-0.909,0-2.026-0.551L2.28,18.675c-0.57-0.329-0.922-0.945-0.922-1.604V6.921 c0-0.659,0.353-1.275,0.922-1.603l8.795-5.082c0.557-0.315,1.296-0.315,1.848,0l8.794,5.082c0.57,0.329,0.924,0.944,0.924,1.603 v10.15c0,0.659-0.354,1.273-0.924,1.604l-8.794,5.078C12.643,23.916,12.324,24,11.998,24z M19.099,13.993 c0-1.9-1.284-2.406-3.987-2.763c-2.731-0.361-3.009-0.548-3.009-1.187c0-0.528,0.235-1.233,2.258-1.233 c1.807,0,2.473,0.389,2.747,1.607c0.024,0.115,0.129,0.199,0.247,0.199h1.141c0.071,0,0.138-0.031,0.186-0.081 c0.048-0.054,0.074-0.123,0.067-0.196c-0.177-2.098-1.571-3.076-4.388-3.076c-2.508,0-4.004,1.058-4.004,2.833 c0,1.925,1.488,2.457,3.895,2.695c2.88,0.282,3.103,0.703,3.103,1.269c0,0.983-0.789,1.402-2.642,1.402 c-2.327,0-2.839-0.584-3.011-1.742c-0.02-0.124-0.126-0.215-0.253-0.215h-1.137c-0.141,0-0.254,0.112-0.254,0.253 c0,1.482,0.806,3.248,4.655,3.248C17.501,17.007,19.099,15.91,19.099,13.993z',
    wasm: 'M14.745,0c0,0.042,0,0.085,0,0.129c0,1.52-1.232,2.752-2.752,2.752c-1.52,0-2.752-1.232-2.752-2.752 c0-0.045,0-0.087,0-0.129H0v24h24V0H14.745z M11.454,21.431l-1.169-5.783h-0.02l-1.264,5.783H7.39l-1.824-8.497h1.59l1.088,5.783 h0.02l1.311-5.783h1.487l1.177,5.854h0.02l1.242-5.854h1.561l-2.027,8.497H11.454z M20.209,21.431l-0.542-1.891h-2.861l-0.417,1.891 h-1.59l2.056-8.497h2.509l2.5,8.497H20.209z M17.812,15.028l-0.694,3.118h2.159l-0.796-3.118H17.812z',
    vite: 'm8.286 10.578.512-8.657a.306.306 0 0 1 .247-.282L17.377.006a.306.306 0 0 1 .353.385l-1.558 5.403a.306.306 0 0 0 .352.385l2.388-.46a.306.306 0 0 1 .332.438l-6.79 13.55-.123.19a.294.294 0 0 1-.252.14c-.177 0-.35-.152-.305-.369l1.095-5.301a.306.306 0 0 0-.388-.355l-1.433.435a.306.306 0 0 1-.389-.354l.69-3.375a.306.306 0 0 0-.37-.36l-2.32.536a.306.306 0 0 1-.374-.316zm14.976-7.926L17.284 3.74l-.544 1.887 2.077-.4a.8.8 0 0 1 .84.369.8.8 0 0 1 .034.783L12.9 19.93l-.013.025-.015.023-.122.19a.801.801 0 0 1-.672.37.826.826 0 0 1-.634-.302.8.8 0 0 1-.16-.67l1.029-4.981-1.12.34a.81.81 0 0 1-.86-.262.802.802 0 0 1-.165-.67l.63-3.08-2.027.468a.808.808 0 0 1-.768-.233.81.81 0 0 1-.217-.6l.389-6.57-7.44-1.33a.612.612 0 0 0-.64.906L11.58 23.691a.612.612 0 0 0 1.066-.004l11.26-20.135a.612.612 0 0 0-.644-.9z',
    // A shell prompt, drawn for the CLI, which has no logo of its own.
    cli: 'M1.8 5.4 3.6 3.6 12 12l-8.4 8.4-1.8-1.8L8.4 12Z M13.2 18h9v2.6h-9Z',
} as const;

const loadFont = async (file: string) => parse((await readFile(require.resolve(file))).buffer);

export const palette = {
    molten: '#ff6a13',
    ember: '#ffb02e',
    mold: { light: '#5a5258', dark: '#b3abba' },
    ink: { light: '#1c1a1f', dark: '#f3ece2' },
};

/** The bracketed-serif T in a 100×86 box; its bottom edge sits on the text baseline. */
export const T_PATH = 'M14 18 H86 V42 Q84 34 76 32 H57 V78 Q57 82 62 82 H68 V86 H32 V82 H38 Q43 82 43 78 V32 H24 Q16 34 14 42 Z';
export const T_BOX = { width: 100, height: 86 };
/** The metal level in the T's box: empty at the stem's foot, full at the crossbar. */
export const LEVEL = { empty: 90, full: 22 };

/** A still frame of the landing page's molten surface, at the full-mold level. */
export function moltenSurface(level = LEVEL.full, amplitude = 1.3, phase = 0): string {
    const points: string[] = [];
    for (let x = 0; x <= 100; x += 4) {
        const y = level + amplitude * Math.sin((x / 25) * Math.PI + phase) + amplitude * 0.5 * Math.sin((x / 16.7) * Math.PI - phase * 1.3);
        points.push(`${x ? 'L' : 'M'}${x} ${y.toFixed(2)}`);
    }
    return points.join(' ');
}

/** The T: metal clipped to the mold, its surface line, and the mold outline. `mold` is a color or, for themed SVGs, a class. */
export function mark(id: string, { mold, strokeWidth, surface = moltenSurface() }: { mold: { color: string } | { className: string }; strokeWidth: number; surface?: string }) {
    const moldPaint = 'color' in mold ? `stroke="${mold.color}"` : `class="${mold.className}"`;
    return [
        `<clipPath id="${id}"><path d="${T_PATH}"/></clipPath>`,
        `<g clip-path="url(#${id})">${metal(surface, strokeWidth)}</g>`,
        `<path fill="none" ${moldPaint} stroke-width="${strokeWidth}" stroke-linejoin="round" d="${T_PATH}"/>`,
    ].join('');
}

export function metal(surface: string, strokeWidth: number): string {
    return `<path fill="${palette.molten}" d="${surface} L150 100 L-50 100 Z"/><path fill="none" stroke="${palette.ember}" stroke-width="${round(strokeWidth * 0.8)}" d="${surface}"/>`;
}

export const zilla = await loadFont('@fontsource/zilla-slab/files/zilla-slab-latin-600-normal.woff');
const SIZE = 100;
export const BASELINE = 0;

export const round = (value: number) => +value.toFixed(2);

// opentype.js's own toPathData() emits `NaN` for some of Zilla Slab's points, so serialize the commands directly.
export function pathData({ commands }: Path): string {
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

export const layout = layoutWordmark();

export const mono = await loadFont('@fontsource/jetbrains-mono/files/jetbrains-mono-latin-500-normal.woff');
export const iron = '#1c1a1f';

/** A line of text as outlines. */
export const text = (font: typeof zilla, value: string, x: number, y: number, size: number, fill: string) =>
    `<path fill="${fill}" d="${pathData(font.getPath(value, x, y, size, { kerning: true }))}"/>`;

/** What a page card shows, besides the title: the tool's mark, or the cast T on pages that belong to every tool. */
export type CardMark = keyof typeof toolMarks | 'fonte';

export interface PageCard {
    /** The plate in the top corner, as the top bar shows it: `npm · Node.js`. */
    plate: string;
    title: string;
    /** A line under the title: the tool's package, or the guide's section. */
    detail: string;
    /** The page's address, shown without its scheme. */
    url: string;
    mark: CardMark;
}

/** The size link previews show uncropped. */
export const pageCardSize = { width: 1200, height: 630 } as const;

const escapeXml = (value: string) => value.replace(/[&<>"]/g, c => `&#${c.charCodeAt(0)};`);

/** The title in Zilla Slab, wrapped to `maxWidth` in two lines at most, at the largest size that fits. */
function wrapTitle(title: string, maxWidth: number): { size: number; lines: string[] } {
    for (const size of [84, 72, 64]) {
        const lines: string[] = [];
        for (const word of title.split(' ')) {
            const last = lines.at(-1);
            if (last && zilla.getAdvanceWidth(`${last} ${word}`, size, { kerning: true }) <= maxWidth) lines[lines.length - 1] = `${last} ${word}`;
            else lines.push(word);
        }
        if (lines.length <= 2 && lines.every(line => zilla.getAdvanceWidth(line, size, { kerning: true }) <= maxWidth)) return { size, lines };
    }
    throw new Error(`The page card title "${title}" doesn't fit in two lines.`);
}

/**
 * A doc page's link-preview card: the page title on ash under the wordmark, with its tool's plate and mark, as on the
 * page itself. Text is outlined, so the card renders the same without fonts.
 */
export function pageCard({ plate, title, detail, url, mark: icon }: PageCard): string {
    const { width, height } = pageCardSize;
    const margin = 72;
    const ink = palette.ink.light;
    const muted = palette.mold.light;
    const markSize = 168;
    const markTop = 360;

    const { size, lines } = wrapTitle(title, width - 2 * margin - markSize - 48);
    const lineHeight = round(size * 1.14);

    const wordmarkHeight = 50;
    const wordmarkScale = wordmarkHeight / (BASELINE - layout.top);
    const rule = (y: number) => `<path stroke="${ink}" stroke-opacity=".18" stroke-width="2" d="M${margin} ${y}H${width - margin}"/>`;
    const plateWidth = mono.getAdvanceWidth(plate, 22);
    // The T's mold spans x 14–86 and y 18–86 of its box; it fills the mark's square from the top.
    const tScale = markSize / 72;
    const drawnMark =
        icon === 'fonte'
            ? `<g transform="translate(${round(width - margin - markSize - 14 * tScale)} ${round(markTop - 18 * tScale)}) scale(${round(tScale)})">${mark('page-mark', { mold: { color: muted }, strokeWidth: 2.4 })}</g>`
            : `<g transform="translate(${width - margin - markSize} ${markTop}) scale(${markSize / 24})"><path fill="${palette.molten}" d="${toolMarks[icon]}"/></g>`;

    return [
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title">`,
        `<title id="title">${escapeXml(`${title}, ${plate}: fonte`)}</title>`,
        `<path fill="#faf7f2" d="M0 0H${width}V${height}H0Z"/>`,
        `<g transform="translate(${margin} ${round(56 + wordmarkHeight)}) scale(${round(wordmarkScale)})"><path fill="${ink}" d="${layout.letters}"/><g transform="${layout.tTransform}">${mark('page-wordmark', { mold: { color: muted }, strokeWidth: 2.4 })}</g></g>`,
        text(mono, plate, round(width - margin - plateWidth), 94, 22, '#b4410a'),
        rule(128),
        ...lines.map((line, i) => text(zilla, line, margin, (lines.length === 1 ? 320 : 320 - lineHeight / 2) + i * lineHeight, size, ink)),
        detail ? text(mono, detail, margin, 452, 24, muted) : '',
        drawnMark,
        rule(556),
        text(mono, url, margin, 596, 22, muted),
        '</svg>\n',
    ].join('');
}
