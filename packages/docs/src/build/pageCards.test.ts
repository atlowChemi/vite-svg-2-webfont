import { describe, expect, it, vi } from 'vite-plus/test';
import { pageCard } from './brand.ts';
import { cardAlt, cardFor, cardPath, renderCard } from './pageCards.ts';

// Cards stay in this test run's memory, out of the real cache on disk.
vi.mock('./diskCache.ts', () => ({
    installed: (name: string) => name,
    DiskCache: class {
        readonly #entries = new Map<string, unknown>();
        getOrCreate = (key: string[], make: () => unknown) => {
            const id = key.join('\0');
            if (!this.#entries.has(id)) this.#entries.set(id, make());
            return this.#entries.get(id);
        };
        prune = () => {};
    },
}));

const docsUrl = 'https://fonte.atlow.dev/';

describe('cardPath', () => {
    it('puts each card beside its page, an index for a section', () => {
        expect(cardPath('/node/configuration')).toBe('og/node/configuration.png');
        expect(cardPath('/node/')).toBe('og/node/index.png');
        expect(cardPath('/guide/features/variants')).toBe('og/guide/features/variants.png');
    });
});

describe('cardFor', () => {
    it("names a tool page by its navigation label, under the tool's plate and package", () => {
        expect(cardFor({ route: '/node/configuration', title: 'Node.js Configuration' }, docsUrl)).toEqual({
            plate: 'npm · Node.js',
            title: 'Configuration',
            detail: '@atlowchemi/webfont-generator',
            url: 'fonte.atlow.dev/node/configuration',
            mark: 'node',
        });
        expect(cardFor({ route: '/browser/', title: 'Browser' }, docsUrl)).toMatchObject({ title: 'Getting Started', mark: 'wasm' });
        expect(cardFor({ route: '/cli/usage', title: 'CLI Usage' }, docsUrl)).toMatchObject({ plate: 'Binary · CLI', mark: 'cli' });
    });

    it("marks guide pages with the T, and names the guide's other sections", () => {
        expect(cardFor({ route: '/guide/glossary', title: 'Glossary' }, docsUrl)).toMatchObject({ plate: 'Guide', detail: '', mark: 'fonte' });
        expect(cardFor({ route: '/guide/features/variants', title: 'Variants' }, docsUrl)).toMatchObject({ plate: 'Guide', detail: 'Features' });
    });

    it('shows the address the site is deployed to', () => {
        expect(cardFor({ route: '/playground', title: 'Playground' }, 'https://pr-1.example.dev/')).toMatchObject({
            plate: 'Try it',
            url: 'pr-1.example.dev/playground',
            mark: 'wasm',
        });
    });

    it("leaves the landing and 404 pages to the site's card", () => {
        expect(cardFor({ route: '/', title: 'fonte' }, docsUrl)).toBeUndefined();
        expect(cardFor({ route: '/404', title: 'Not found' }, docsUrl)).toBeUndefined();
    });
});

describe('pageCard', () => {
    const card = { plate: 'Guide', title: 'SVG Authoring & Troubleshooting', detail: '', url: 'fonte.atlow.dev/guide/svg-authoring', mark: 'fonte' } as const;

    it('outlines its text, escaping only the title element', () => {
        const svg = pageCard(card);
        expect(svg).toContain('<title id="title">SVG Authoring &#38; Troubleshooting, Guide: fonte</title>');
        expect(svg).not.toContain('<text');
        expect(cardAlt(card)).toBe('SVG Authoring & Troubleshooting, Guide: fonte');
    });

    it("fails on a title that doesn't fit in two lines", () => {
        expect(() => pageCard({ ...card, title: 'A title far too long to fit on a link preview card in two lines at any size' })).toThrow(/doesn't fit/);
    });

    it('renders to a 1200×630 PNG', () => {
        const png = Buffer.from(renderCard(card));
        expect(png.subarray(1, 4).toString()).toBe('PNG');
        expect([png.readUInt32BE(16), png.readUInt32BE(20)]).toEqual([1200, 630]);
    });
});
