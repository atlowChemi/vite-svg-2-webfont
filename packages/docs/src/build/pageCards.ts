import { Resvg } from '@resvg/resvg-js';
import { pageNav } from '../site/nav.ts';
import { tools } from '../site/site.ts';
import { pageCard, pageCardSize, type CardMark, type PageCard } from './brand.ts';
import { DiskCache, installed } from './diskCache.ts';

// Each doc page's link preview: its title, tool and address, drawn by `pageCard` and rendered to PNG at build time.
// The landing and 404 pages keep the site's own card.

const marks: Record<string, CardMark> = { vite: 'vite', node: 'node', browser: 'wasm', cli: 'cli', rust: 'rust' };

/** The card's path in the built site, beside the pages: `/node/configuration` → `og/node/configuration.png`. */
export const cardPath = (route: string) => `og${route.endsWith('/') ? `${route}index` : route}.png`;

/** What a page's card shows; none for the landing and 404 pages. */
export function cardFor(page: { route: string; title: string }, docsUrl: string): PageCard | undefined {
    if (page.route === '/' || page.route === '/404') return undefined;
    const url = new URL(page.route.slice(1), docsUrl).href.replace(/^https?:\/\//, '');
    if (page.route === '/playground') {
        const browser = tools.find(t => t.key === 'browser')!;
        return { plate: 'Try it', title: page.title, detail: browser.package, url, mark: 'wasm' };
    }
    const nav = pageNav(page.route);
    // The page's name in the navigation: `Configuration`, where the plate already names the tool.
    const title = nav.group?.items[nav.index]?.text ?? page.title;
    if (nav.tool) return { plate: `${nav.tool.plate} · ${nav.tool.name}`, title, detail: nav.tool.package, url, mark: marks[nav.tool.key]! };
    // The guide's own section repeats the plate; its others name themselves (`Features`).
    const group = nav.group?.text;
    return { plate: 'Guide', title, detail: group && group !== 'Guide' ? group : '', url, mark: 'fonte' };
}

/** The alt text for a card: what it shows, as its SVG's title. */
export const cardAlt = (card: PageCard) => `${card.title}, ${card.plate}: fonte`;

// A card's PNG follows from its content, the brand's shapes and fonts, and resvg.
const stored = new DiskCache<Uint8Array>('page-cards', ['src/build/brand.ts', 'src/build/pageCards.ts'], {
    packages: ['@resvg/resvg-js', 'opentype.js', '@fontsource/zilla-slab', '@fontsource/jetbrains-mono'].map(installed),
});

/** After a full build: drops the stored cards no page used, and writes the rest. */
export const prunePageCards = () => stored.prune();

// The SVG's text is outlined, so no fonts load; skipping the system's keeps renders the same on every machine.
const font = { loadSystemFonts: false };

/** The card as a PNG. */
export function renderCard(card: PageCard): Uint8Array {
    return stored.getOrCreate([JSON.stringify(card)], () => {
        const svg = pageCard(card);
        return new Resvg(svg, { font, fitTo: { mode: 'width', value: pageCardSize.width } }).render().asPng();
    });
}
