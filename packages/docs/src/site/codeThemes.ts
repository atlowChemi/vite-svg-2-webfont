import type { ThemeRegistrationAny } from 'shiki/core';

/** Code block themes, shared by the build's highlighter and the playground's client-side one. */
export const codeThemes = { light: 'catppuccin-latte', dark: 'catppuccin-frappe' } as const;

/**
 * What code is drawn on: the blocks' background (`--surface-sunken` in tokens.css, which replaces the themes' own) and a
 * highlighted line, `--tint` over it.
 */
export const codeBackgrounds = { light: ['#f3ece2', '#e7e0d7'], dark: ['#161418', '#23242a'] } as const;

type Rgb = [number, number, number];
const parse = (hex: string): Rgb => [0, 2, 4].map(i => Number.parseInt(hex.slice(1 + i, 3 + i), 16)) as Rgb;
const format = (rgb: Rgb) => `#${rgb.map(c => Math.round(c).toString(16).padStart(2, '0')).join('')}`;
const luminance = (rgb: Rgb) => {
    const [r, g, b] = rgb.map(c => ((c /= 255) <= 0.039_28 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)) as Rgb;
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const contrast = (a: Rgb, b: Rgb) => {
    const [hi, lo] = [luminance(a), luminance(b)].toSorted((x, y) => y - x) as [number, number];
    return (hi + 0.05) / (lo + 0.05);
};

/** Darkens (on light backgrounds) or lightens a color in small steps until it reaches `ratio` against each background. */
function readable(color: string, backgrounds: Rgb[], ratio: number): string {
    if (!/^#[\da-f]{6}([\da-f]{2})?$/i.test(color)) return color;
    const alpha = color.slice(7);
    const target: Rgb = luminance(backgrounds[0]!) > 0.5 ? [0, 0, 0] : [255, 255, 255];
    let rgb = parse(color);
    for (let step = 0; step < 40 && Math.min(...backgrounds.map(bg => contrast(rgb, bg))) < ratio; step++) {
        rgb = rgb.map((c, i) => c + (target[i]! - c) * 0.05) as Rgb;
    }
    return format(rgb) + alpha;
}

/**
 * Catppuccin's accents are tuned for its own backgrounds; on the docs' cream and iron some fall below WCAG's 4.5:1 for
 * text. Each foreground is nudged toward black or white just enough to reach it, so the palette keeps its hues.
 */
export function withReadableTokens(theme: ThemeRegistrationAny, backgrounds: readonly string[], ratio = 4.5): ThemeRegistrationAny {
    const bg = backgrounds.map(parse);
    return {
        ...theme,
        colors: Object.fromEntries(Object.entries(theme.colors ?? {}).map(([key, value]) => [key, key.endsWith('foreground') ? readable(value, bg, ratio) : value])),
        tokenColors: theme.tokenColors?.map(rule =>
            rule.settings?.foreground ? { ...rule, settings: { ...rule.settings, foreground: readable(rule.settings.foreground, bg, ratio) } } : rule,
        ),
    };
}
