import { describe, expect, it } from 'vitest';
import MiniSearch from 'minisearch';
import { indexOptions, type SearchDoc } from '../client/searchOptions.ts';
import { llmsFullTxt, llmsTxt, markdownFiles, parseRendered, plainMarkdown, searchIndex, searchSections, type MarkdownBlocks } from './outputs.ts';
import type { Page } from './render.tsx';

const components: MarkdownBlocks = { names: new Set(['ExampleTabs', 'ExampleIcons', 'ExampleOutput']), visual: new Set(['ExampleOutput']) };
const page = (route: string, title: string, source = '', description?: string) =>
    ({
        route,
        file: `${route.slice(1) || 'index'}.md`,
        title,
        description,
        source,
        frontmatter: {},
        headings: [],
        html: '',
        sections: [],
        deps: new Set(),
        entries: new Set(),
    }) as Page;

describe('plainMarkdown', () => {
    it('keeps the Markdown in component slots and drops the components', () => {
        const source = [
            'Before.',
            '',
            `<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/' }">`,
            '<template #config>',
            '',
            '```ts',
            'export default {};',
            '```',
            '',
            '</template>',
            '<template #icons>',
            '<ExampleIcons example="basic" />',
            '</template>',
            '</ExampleTabs>',
            '',
            'After.',
        ].join('\n');
        expect(plainMarkdown(source, components)).toBe('Before.\n\n```ts\nexport default {};\n```\n\nAfter.');
    });

    it('drops visual-only components with their slots, keeping the Markdown around them', () => {
        const source = [
            'The font:',
            '',
            '<ExampleOutput example="ligatures">',
            '<figure><span>add check</span><figcaption>text: add check</figcaption></figure>',
            '</ExampleOutput>',
            '',
            'After.',
        ].join('\n');
        expect(plainMarkdown(source, components)).toBe('The font:\n\nAfter.');
    });

    it('drops custom heading ids', () => {
        expect(plainMarkdown('### `dispose()` {#generator-dispose}', components)).toBe('### `dispose()`');
    });

    it('drops HTML tags in prose but keeps their text, and leaves code alone', () => {
        const source = [
            '<figure><i class="icon icon-bell"></i><figcaption>bell</figcaption></figure>',
            '',
            'Use `<i class="icon">` here.',
            '',
            '```html',
            '<i class="icon icon-add"></i>',
            '```',
        ].join('\n');
        expect(plainMarkdown(source, components)).toBe(['bell', '', 'Use `<i class="icon">` here.', '', '```html', '<i class="icon icon-add"></i>', '```'].join('\n'));
    });
});

describe('llmsTxt', () => {
    it('lists pages in sidebar order under their groups, with the rest last', () => {
        const pages = [
            page('/vite/usage', 'Usage'),
            page('/guide/engine', 'The Engine'),
            page('/playground', 'Playground'),
            page('/guide/', 'Choosing a Tool', '', 'Pick one.'),
            page('/vite/', 'Getting Started'),
        ];
        expect(llmsTxt(pages)).toContain(
            [
                '## Table of Contents',
                '',
                '### Guide',
                '',
                '- [Choosing a Tool](/guide/index.md): Pick one.',
                '- [The Engine](/guide/engine.md)',
                '',
                '### Vite Plugin',
                '',
                '- [Getting Started](/vite/index.md)',
                '- [Usage](/vite/usage.md)',
                '',
                '### More',
                '',
                '- [Playground](/playground.md)',
            ].join('\n'),
        );
    });
});

describe('markdownFiles', () => {
    it('starts each page with its title, description and URL', () => {
        const files = markdownFiles([page('/guide/engine', 'The Engine', '# The Engine', 'How it works.')], components);
        expect(files.get('guide/engine.md')).toBe('---\ntitle: "The Engine"\ndescription: "How it works."\nurl: /guide/engine.md\n---\n\n# The Engine\n');
    });

    it('points an index page at its Markdown copy, as llms.txt does', () => {
        const files = markdownFiles([page('/guide/', 'Choosing a Tool', '# Choosing a Tool')], components);
        expect(files.get('guide/index.md')).toContain('\nurl: /guide/index.md\n');
    });
});

describe('llmsFullTxt', () => {
    it('closes each page with a rule before the next one starts', () => {
        const full = llmsFullTxt([page('/guide/', 'Choosing a Tool', '# Choosing a Tool'), page('/guide/engine', 'The Engine', '# The Engine')], components);
        expect(full).toBe(
            [
                '---',
                'title: "Choosing a Tool"',
                'url: /guide/index.md',
                '---',
                '',
                '# Choosing a Tool',
                '',
                '---',
                '',
                '---',
                'title: "The Engine"',
                'url: /guide/engine.md',
                '---',
                '',
                '# The Engine',
                '',
            ].join('\n'),
        );
    });
});

const index = (html: string) =>
    MiniSearch.loadJSON<SearchDoc>(searchIndex([{ ...page('/guide/engine', 'The Engine'), sections: searchSections(parseRendered(html), '/guide/engine') }]), indexOptions);

describe('searchIndex', () => {
    it('starts a section at every heading level, h4 included', () => {
        const search = index('<h2 id="a">A</h2><p>alpha</p><h4 id="b">B</h4><p>beta</p>');
        expect(search.search('beta')[0]).toMatchObject({ id: '/guide/engine#b', title: 'B', titles: ['A'] });
        expect(search.search('alpha')[0]).toMatchObject({ id: '/guide/engine#a', title: 'A' });
    });

    it('keeps adjacent links apart in the searched text', () => {
        const html = '<h2 id="a">A</h2><p>Also in <a href="/x">Browser</a><a href="/y">CLI</a></p>';
        expect(searchSections(parseRendered(html), '/guide/engine')[0]!.text).toBe('Also in Browser CLI');
        expect(index(html).search('cli')[0]!.id).toBe('/guide/engine#a');
    });

    it("stores no section's text: excerpts come from the page itself", () => {
        expect(index('<h2 id="a">A</h2><p>alpha</p>').search('alpha')[0]).not.toHaveProperty('text');
    });
});
