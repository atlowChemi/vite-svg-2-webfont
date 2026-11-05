import { join } from 'node:path';
import { describe, expect, it } from 'vite-plus/test';
import styles from './markdown.module.css';
import { renderMarkdown } from './markdown.ts';

const render = () => renderMarkdown(join(import.meta.dirname, 'fixtures/markdown.md'), () => '', new Set());

describe('renderMarkdown', () => {
    it('takes a heading id from a trailing {#id}, without showing it', async () => {
        const { html, headings } = await render();
        expect(html).toContain(`<h2 id="generator-dispose" tabindex="-1"><code>dispose()</code> <a class="${styles.anchor}" href="#generator-dispose"`);
        expect(html).not.toContain('{#');
        expect(headings[0]).toMatchObject({ title: 'dispose()', slug: 'generator-dispose' });
    });

    it('still numbers repeated headings', async () => {
        const { headings } = await render();
        expect(headings.slice(1).map(h => h.slug)).toEqual(['plain-heading', 'plain-heading-1']);
    });

    it('opens external links in a new tab, and only those', async () => {
        const { html } = await render();
        expect(html).toContain('<a href="https://github.com/sunflowerdeath/webfonts-generator" target="_blank" rel="noreferrer">');
        expect(html).toContain('<a href="//example.com/x" target="_blank" rel="noreferrer">');
        expect(html).toContain('<a href="/guide/">');
        expect(html).toContain('<a href="#plain-heading">');
    });

    it('lets tables take focus, to scroll from the keyboard', async () => {
        const { html } = await render();
        expect(html).toContain('<table tabindex="0">');
    });
});
