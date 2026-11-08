import type { Browser } from 'playwright';
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { diagramBrowser } from './diagramBrowser.ts';
import styles from './mermaid.module.css';
import { diagramKey, renderDiagrams, themed } from './mermaid.ts';

interface RenderArgs {
    id: string;
    code: string;
    themeVariables: Record<string, string>;
}

const { launch } = vi.hoisted(() => ({ launch: vi.fn<() => Promise<Browser>>() }));
vi.mock('playwright', () => ({ chromium: { launch } }));

/** A stand-in Chromium whose tabs "render" a diagram as an SVG drawn in the placeholder colors they were given. */
function fakeBrowser() {
    const tabs: { close: ReturnType<typeof vi.fn> }[] = [];
    const evaluate = vi.fn(async (_render: unknown, { id, code, themeVariables }: RenderArgs) => {
        if (code === 'broken') throw new Error('Parse error');
        return `<svg id="${id}"><rect fill="${themeVariables['primaryColor']}"/><text>${code}</text></svg>`;
    });
    const browser = {
        tabs,
        evaluate,
        close: vi.fn(async () => {}),
        newPage: vi.fn(async () => {
            const tab = { goto: vi.fn(async () => null), close: vi.fn(async () => {}), evaluate };
            tabs.push(tab);
            return { ...tab, [Symbol.asyncDispose]: () => tab.close() };
        }),
    };
    return browser;
}

let browser: ReturnType<typeof fakeBrowser>;
beforeEach(() => {
    launch.mockReset();
    browser = fakeBrowser();
    launch.mockImplementation(async () => browser as unknown as Browser);
});

// The cache outlives a test, so each test draws diagrams of its own.
const sources = (...codes: string[]) => new Map(codes.map(code => [diagramKey(code), code]));
const collect = (diagrams: Map<string, string>) => Array.fromAsync(renderDiagrams(diagrams));

describe('renderDiagrams', () => {
    it('yields each diagram in order, themed with the site tokens', async () => {
        const diagrams = sources('graph LR; order-a', 'graph LR; order-b');
        const rendered = await collect(diagrams);
        expect(rendered.map(([key]) => key)).toEqual([...diagrams.keys()]);
        const [, html] = rendered[0]!;
        expect(html).toMatch(new RegExp(`^<div class="${styles.diagram}"><svg`));
        expect(html).toContain('fill="var(--surface-muted)"');
        expect(html).toContain('graph LR; order-a');
    });

    it('renders a page in one tab, and closes it and the browser when done', async () => {
        await collect(sources('graph LR; one-tab-a', 'graph LR; one-tab-b'));
        expect(browser.tabs).toHaveLength(1);
        expect(browser.evaluate).toHaveBeenCalledTimes(2);
        expect(browser.tabs[0]!.close).toHaveBeenCalledOnce();
        expect(browser.close).toHaveBeenCalledOnce();
    });

    it('needs no browser for diagrams it has rendered before', async () => {
        const diagrams = sources('graph LR; cached');
        const first = await collect(diagrams);
        launch.mockClear();
        expect(await collect(diagrams)).toEqual(first);
        expect(launch).not.toHaveBeenCalled();
    });

    it('leaves the browser open while someone else holds it', async () => {
        const held = diagramBrowser.use();
        await collect(sources('graph LR; held'));
        expect(browser.tabs[0]!.close).toHaveBeenCalledOnce();
        expect(browser.close).not.toHaveBeenCalled();
        await held[Symbol.asyncDispose]();
        expect(browser.close).toHaveBeenCalledOnce();
    });

    it('closes the tab and the browser when the caller stops early', async () => {
        for await (const _diagram of renderDiagrams(sources('graph LR; early-a', 'graph LR; early-b'))) break;
        expect(browser.evaluate).toHaveBeenCalledOnce();
        expect(browser.tabs[0]!.close).toHaveBeenCalledOnce();
        expect(browser.close).toHaveBeenCalledOnce();
    });

    it('closes the tab and the browser when a diagram fails', async () => {
        await expect(collect(sources('broken'))).rejects.toThrow('Parse error');
        expect(browser.tabs[0]!.close).toHaveBeenCalledOnce();
        expect(browser.close).toHaveBeenCalledOnce();
    });
});

// `#0d0000` is the first placeholder, Mermaid's `background`, which becomes `--surface`.
describe('themed', () => {
    it('swaps placeholder colors for the site tokens, in any case', () => {
        expect(themed('<svg><rect fill="#0D0000"/></svg>', 'k')).toBe('<svg><rect fill="var(--surface)"/></svg>');
    });

    it('mixes translucent placeholders with transparent', () => {
        expect(themed('<svg><rect fill="rgba(13, 0, 0, 0.5)"/></svg>', 'k')).toBe('<svg><rect fill="color-mix(in srgb, var(--surface) 50%, transparent)"/></svg>');
    });

    it('drops the KaTeX and hand-drawn rules, and the filters only they used', () => {
        const svg = [
            '<svg><style>',
            '#m .node{fill:#0d0000}',
            '.katex .x{color:#123456}',
            '#m .edge,[data-look="neo"] .edge{stroke:#0d0000}',
            '[data-look="neo"] .shadow{filter:url(#drop-shadow)}',
            '</style>',
            '<filter id="drop-shadow"><feFlood flood-color="#123456"/></filter>',
            '<filter id="used"><feGaussianBlur/></filter><g filter="url(#used)"/>',
            '</svg>',
        ].join('');
        expect(themed(svg, 'k')).toBe(
            '<svg><style>#m .node{fill:var(--surface)}#m .edge{stroke:var(--surface)}</style><filter id="used"><feGaussianBlur/></filter><g filter="url(#used)"/></svg>',
        );
    });

    it('throws on colors with no site token, naming the diagram', () => {
        expect(() => themed('<svg><rect fill="#123456"/><rect stroke="hsl(0, 0%, 0%)"/></svg>', 'abc')).toThrow(
            'Mermaid diagram abc has colors with no site token: #123456, hsl(0, 0%, 0%)',
        );
    });
});
