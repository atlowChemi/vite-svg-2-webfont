import { createHash } from 'node:crypto';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

// Mermaid lays text out against a real DOM, so diagrams render in headless Chromium at build time and readers download no
// Mermaid at all. Mermaid derives shades from its theme colors and rejects `var(--…)`, so each diagram is drawn once with
// placeholder colors that are then swapped for the site's tokens: the SVG follows the light and dark themes through CSS.

export const diagramKey = (code: string) => createHash('sha256').update(code.trim()).digest('hex').slice(0, 12);
export const marker = (key: string) => `<!--fonte-mermaid:${key}-->`;

// Each theme color Mermaid uses, as a placeholder and the token that replaces it. Nodes are drawn like the type case's
// compartments, edges in the brand color.
const tokens: Record<string, string> = {
    background: '--vp-c-bg',
    primaryColor: '--vp-c-bg-soft',
    primaryBorderColor: '--fonte-mold',
    primaryTextColor: '--vp-c-text-1',
    secondaryColor: '--vp-c-bg-soft',
    secondaryBorderColor: '--fonte-mold',
    secondaryTextColor: '--vp-c-text-1',
    tertiaryColor: '--vp-c-bg-alt',
    tertiaryBorderColor: '--vp-c-divider',
    tertiaryTextColor: '--vp-c-text-1',
    mainBkg: '--vp-c-bg-soft',
    nodeBorder: '--fonte-mold',
    nodeTextColor: '--vp-c-text-1',
    textColor: '--vp-c-text-1',
    lineColor: '--vp-c-brand-1',
    arrowheadColor: '--vp-c-brand-1',
    edgeLabelBackground: '--vp-c-bg',
    clusterBkg: '--vp-c-bg-alt',
    clusterBorder: '--vp-c-divider',
    titleColor: '--vp-c-text-2',
    labelBackground: '--vp-c-bg',
    actorBkg: '--vp-c-bg-soft',
    actorBorder: '--fonte-mold',
    actorTextColor: '--vp-c-text-1',
    actorLineColor: '--vp-c-divider',
    signalColor: '--vp-c-brand-1',
    signalTextColor: '--vp-c-text-1',
    labelBoxBkgColor: '--vp-c-bg-soft',
    labelBoxBorderColor: '--fonte-mold',
    labelTextColor: '--vp-c-text-1',
    loopTextColor: '--vp-c-text-1',
    noteBkgColor: '--vp-c-bg-alt',
    noteBorderColor: '--vp-c-divider',
    noteTextColor: '--vp-c-text-1',
    activationBkgColor: '--vp-c-bg-alt',
    activationBorderColor: '--vp-c-divider',
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

export async function renderDiagrams(sources: Map<string, string>): Promise<Map<string, string>> {
    const missing = [...sources].filter(([key]) => !cache.has(key));
    if (missing.length) {
        const browser = await chromium.launch();
        try {
            const page = await browser.newPage();
            await page.addScriptTag({ path: join(dirname(fileURLToPath(import.meta.resolve('mermaid'))), 'mermaid.min.js') });
            for (const [key, source] of missing) {
                // One page renders the diagrams in turn: Mermaid keeps global state between renders.
                // oxlint-disable-next-line no-await-in-loop
                const svg = await page.evaluate(
                    async ({ id, code, themeVariables }) => {
                        const { mermaid } = globalThis as unknown as { mermaid: typeof import('mermaid').default };
                        // SVG labels avoid HTML text being clipped by inherited page typography.
                        mermaid.initialize({ startOnLoad: false, securityLevel: 'loose', htmlLabels: false, theme: 'base', themeVariables });
                        return (await mermaid.render(id, code)).svg;
                    },
                    { id: `mermaid-${key}`, code: source, themeVariables: { fontFamily: 'var(--vp-font-family-base)', fontSize: '14px', ...placeholders } },
                );
                cache.set(key, `<div class="fonte-mermaid">${themed(svg, key)}</div>`);
            }
        } finally {
            await browser.close();
        }
    }
    return new Map([...sources.keys()].map(key => [key, cache.get(key)!]));
}
