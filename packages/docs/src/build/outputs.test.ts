import { describe, expect, it } from 'vite-plus/test';
import MiniSearch from 'minisearch';
import { indexOptions, type SearchDoc } from '../client/searchOptions.ts';
import { llmsTxt, markdownFiles, plainMarkdown, searchIndex } from './outputs.ts';
import type { Page } from './render.tsx';

const components = new Set(['ExampleTabs', 'ExampleIcons', 'ExampleOutput']);
const page = (route: string, title: string, source = '', description?: string) =>
    ({ route, file: `${route.slice(1) || 'index'}.md`, title, description, source, frontmatter: {}, headings: [], html: '', deps: new Set(), entries: new Set() }) as Page;

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
        expect(files.get('guide/engine.md')).toBe('---\ntitle: "The Engine"\ndescription: "How it works."\nurl: /guide/engine\n---\n\n# The Engine\n');
    });
});

const index = (html: string) => MiniSearch.loadJSON<SearchDoc>(searchIndex([{ ...page('/guide/engine', 'The Engine'), html }]), indexOptions);

describe('searchIndex', () => {
    it('starts a section at every heading level, h4 included', () => {
        const search = index('<h2 id="a">A</h2><p>alpha</p><h4 id="b">B</h4><p>beta</p>');
        expect(search.search('beta')[0]).toMatchObject({ id: '/guide/engine#b', titles: ['A'], text: 'beta' });
        expect(search.search('alpha')[0]).toMatchObject({ id: '/guide/engine#a', text: 'alpha' });
    });

    it('keeps adjacent links apart in the stored text', () => {
        const search = index('<h2 id="a">A</h2><p>Also in <a href="/x">Browser</a><a href="/y">CLI</a></p>');
        expect(search.search('cli')[0]!.text).toBe('Also in Browser CLI');
    });
});
