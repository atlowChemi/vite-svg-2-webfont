// The site's own icon font: fonte casts the SVGs in `src/fonts/site-icons/` into it during the docs build (see `siteIconFont.ts`).
// home, star, bolt, bell, search, heart, arrow-down, chevron-down: Tabler Icons 3.19.0 (outline), MIT license,
// https://tabler.io/icons. They stay stroked; the engine expands strokes into filled outlines.
// rust, node, wasm, vite: Simple Icons 13.0.0, CC0 1.0, https://simpleicons.org.
// play, github, theme: drawn for this site.

/** Pinned codepoints, keyed by file name. The landing's tray icons come first, so the hero's `U+E001`–`U+E006` are real. */
export const siteIcons = {
    home: 0xe001,
    star: 0xe002,
    bolt: 0xe003,
    bell: 0xe004,
    search: 0xe005,
    heart: 0xe006,
    'arrow-down': 0xe007,
    'chevron-down': 0xe008,
    play: 0xe009,
    github: 0xe00a,
    theme: 0xe00b,
    rust: 0xe00c,
    node: 0xe00d,
    wasm: 0xe00e,
    vite: 0xe00f,
} as const;

export type SiteIcon = keyof typeof siteIcons;

/** The icons the landing hero casts, in codepoint order. */
export const trayIcons = ['home', 'star', 'bolt', 'bell', 'search', 'heart'] as const satisfies readonly SiteIcon[];

/** Classes for an `<i>` that draws the glyph; pair it with `aria-hidden="true"`, the glyph is decoration. */
export const siteIcon = (name: SiteIcon) => `site-icon site-icon-${name}`;

/** A codepoint as the case and the hero label it, e.g. `E001`. */
export const codepointLabel = (name: SiteIcon) => siteIcons[name].toString(16).toUpperCase();
