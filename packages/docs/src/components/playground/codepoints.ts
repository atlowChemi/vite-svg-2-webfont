/** The engine's default start codepoint, in the Private Use Area. */
export const DEFAULT_START_CODEPOINT = 0xf101;

/** The first codepoint from `start` that is not already used, matching how the engine assigns unpinned icons. */
export function nextCodepoint(start: number, used: ReadonlySet<number>): number {
    let codepoint = start;
    while (used.has(codepoint)) codepoint++;
    return codepoint;
}

/** Parse `f101`, `F101`, `0xF101`, `U+F101`, or `\f101`; undefined for anything that is not a Unicode scalar value. */
export function parseCodepoint(text: string): number | undefined {
    const hex = text.trim().replace(/^(?:0x|u\+|\\)/i, '');
    if (!/^[0-9a-f]{1,6}$/i.test(hex)) return undefined;
    const codepoint = Number.parseInt(hex, 16);
    const surrogate = codepoint >= 0xd800 && codepoint <= 0xdfff;
    return codepoint <= 0x10ffff && !surrogate ? codepoint : undefined;
}

export function formatCodepoint(codepoint: number): string {
    return `U+${codepoint.toString(16).toUpperCase().padStart(4, '0')}`;
}

/** Whether the codepoint is in one of Unicode's Private Use Areas, where icon codepoints cannot clash with text. */
export function isPrivateUse(codepoint: number): boolean {
    return (codepoint >= 0xe000 && codepoint <= 0xf8ff) || (codepoint >= 0xf0000 && codepoint <= 0xffffd) || (codepoint >= 0x100000 && codepoint <= 0x10fffd);
}

/** A `codepoints` option that pins the given icons, ready to paste into a plugin or generator config. */
export function codepointsSnippet(icons: readonly { name: string; codepoint: number }[]): string {
    const entries = icons.map(({ name, codepoint }) => {
        const key = /^[A-Za-z_$][\w$]*$/.test(name) ? name : `'${name}'`;
        return `    ${key}: 0x${codepoint.toString(16)},`;
    });
    return `codepoints: {\n${entries.join('\n')}\n},`;
}
