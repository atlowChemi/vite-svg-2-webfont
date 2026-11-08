import { createHash } from 'node:crypto';
import { diagramBrowser } from './diagramBrowser.ts';
import styles from './mermaid.module.css';

// Mermaid lays text out against a real DOM, so diagrams render in headless Chromium at build time and readers download no
// Mermaid at all. Mermaid derives shades from its theme colors and rejects `var(--…)`, so each diagram is drawn once with
// placeholder colors that are then swapped for the site's tokens: the SVG follows the light and dark themes through CSS.

export const diagramKey = (code: string) => createHash('sha256').update(code.trim()).digest('hex').slice(0, 12);
export const marker = (key: string) => `<!--fonte-mermaid:${key}-->`;

// Each theme color Mermaid uses, as a placeholder and the token that replaces it. Nodes are drawn like the type case's
// compartments, edges in the brand color.
const tokens: Record<string, string> = {
    background: '--surface',
    primaryColor: '--surface-muted',
    primaryBorderColor: '--fonte-mold',
    primaryTextColor: '--text',
    secondaryColor: '--surface-muted',
    secondaryBorderColor: '--fonte-mold',
    secondaryTextColor: '--text',
    tertiaryColor: '--surface-sunken',
    tertiaryBorderColor: '--line',
    tertiaryTextColor: '--text',
    mainBkg: '--surface-muted',
    nodeBorder: '--fonte-mold',
    nodeTextColor: '--text',
    textColor: '--text',
    lineColor: '--accent-text',
    arrowheadColor: '--accent-text',
    edgeLabelBackground: '--surface',
    clusterBkg: '--surface-sunken',
    clusterBorder: '--line',
    titleColor: '--text-muted',
    labelBackground: '--surface',
    actorBkg: '--surface-muted',
    actorBorder: '--fonte-mold',
    actorTextColor: '--text',
    actorLineColor: '--line',
    signalColor: '--accent-text',
    signalTextColor: '--text',
    labelBoxBkgColor: '--surface-muted',
    labelBoxBorderColor: '--fonte-mold',
    labelTextColor: '--text',
    loopTextColor: '--text',
    noteBkgColor: '--surface-sunken',
    noteBorderColor: '--line',
    noteTextColor: '--text',
    activationBkgColor: '--surface-sunken',
    activationBorderColor: '--line',
};
const placeholder = (i: number) => `#${(0x0d0000 + i * 0x010101).toString(16).padStart(6, '0')}`;
const placeholders = Object.fromEntries(Object.keys(tokens).map((name, i) => [name, placeholder(i)]));
const byColor = new Map(Object.entries(tokens).map(([name, token]) => [placeholders[name]!, `var(${token})`]));
const rgb = (hex: string) => [1, 3, 5].map(i => Number.parseInt(hex.slice(i, i + 2), 16)).join(',');
const byRgb = new Map([...byColor].map(([hex, value]) => [rgb(hex), value]));

// Mermaid's stylesheet also carries rules for KaTeX labels and its hand-drawn "neo" look, with fixed colors; the docs
// use neither, so those rules and the shadow filters only they reference are dropped.
const rule = /(?<=[}>])([^{}<]+)(\{[^{}]*\})/g;
const unused = (selector: string) => /\.katex|\[data-look="neo"\]/.test(selector);
function withoutUnused(svg: string): string {
    const styled = svg.replace(rule, (match, selectors: string, body: string) => {
        if (!unused(selectors)) return match;
        const kept = selectors.split(',').filter(selector => !unused(selector));
        return kept.length ? `${kept.join(',')}${body}` : '';
    });
    return styled.replace(/<filter id="([^"]+)"[\s\S]*?<\/filter>/g, (filter, id: string) => (styled.includes(`url(#${id})`) ? filter : ''));
}

/** The placeholders swapped for tokens; throws on any color left, so a Mermaid upgrade can't sneak fixed colors back in. */
export function themed(svg: string, key: string): string {
    const out = withoutUnused(svg)
        .replace(/#[\da-f]{6}\b/gi, hex => byColor.get(hex.toLowerCase()) ?? hex)
        .replace(/rgba\((\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)/g, (match, r: string, g: string, b: string, a: string) => {
            const value = byRgb.get(`${r},${g},${b}`);
            return value ? `color-mix(in srgb, ${value} ${Number(a) * 100}%, transparent)` : match;
        });
    // Fixed colors in the diagram's own stylesheet and attributes; `none`, `currentColor` and the tokens are fine.
    const left = [...new Set(out.match(/#[\da-f]{3,8}\b|rgba?\([^)]*\)|hsla?\([^)]*\)/gi) ?? [])].filter(c => !/^#mermaid/i.test(c));
    if (left.length) throw new Error(`Mermaid diagram ${key} has colors with no site token: ${left.join(', ')}`);
    return out;
}

const cache = new Map<string, string>();

/** Each diagram's HTML, by key, in the order given: from the cache, or rendered in a tab of the shared browser. */
export async function* renderDiagrams(sources: Map<string, string>): AsyncGenerator<[string, string], void, undefined> {
    // The browser and a tab only when a diagram needs rendering. However the generator ends, the tab closes, then the
    // browser is let go (disposal runs in reverse), closing it if no one else holds it.
    const uncached = sources.keys().some(key => !cache.has(key));
    await using _browser = uncached ? diagramBrowser.use() : undefined;
    await using tab = uncached ? await diagramBrowser.tab() : undefined;
    for (const [key, source] of sources) {
        if (!cache.has(key)) {
            // The tab renders the page's diagrams in turn: Mermaid keeps global state between renders.
            // oxlint-disable-next-line no-await-in-loop
            const svg = await tab!.evaluate(
                async ({ id, code, themeVariables }) => {
                    const { mermaid } = globalThis as unknown as { mermaid: typeof import('mermaid').default };
                    // SVG labels avoid HTML text being clipped by inherited page typography.
                    mermaid.initialize({ startOnLoad: false, securityLevel: 'loose', htmlLabels: false, theme: 'base', themeVariables });
                    return (await mermaid.render(id, code)).svg;
                },
                { id: `mermaid-${key}`, code: source, themeVariables: { fontFamily: 'var(--font-sans)', fontSize: '14px', ...placeholders } },
            );
            cache.set(key, `<div class="${styles.diagram}">${themed(svg, key)}</div>`);
        }
        yield [key, cache.get(key)!];
    }
}
