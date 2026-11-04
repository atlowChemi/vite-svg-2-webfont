import { parseHTML } from 'linkedom';
import MiniSearch from 'minisearch';
import { indexOptions, type SearchDoc } from '../client/searchOptions.ts';
import type { Page } from './render.tsx';
import { site } from '../site/site.ts';

// Everything derived from the rendered pages in the same pass: the search index and llms.txt / llms-full.txt.

const skip = new Set(['BUTTON', 'SCRIPT', 'STYLE', 'svg', 'SVG']);
function sections(page: Page): SearchDoc[] {
    const { document } = parseHTML(`<!doctype html><html><body>${page.html}</body></html>`);
    const docs: SearchDoc[] = [];
    const trail: { level: number; title: string }[] = [];
    let current: SearchDoc | undefined;
    const walk = (node: Node) => {
        for (const child of node.childNodes) {
            if (child.nodeType === 3) {
                if (current) current.text += child.textContent;
                continue;
            }
            if (child.nodeType !== 1) continue;
            const el = child as Element;
            if (skip.has(el.tagName) || el.classList.contains('header-anchor') || el.classList.contains('lang')) continue;
            const level = /^H([1-3])$/.exec(el.tagName)?.[1];
            if (level && el.id) {
                const title = (el.textContent ?? '').replace(/​/g, '').trim();
                while (trail.length && trail.at(-1)!.level >= Number(level)) trail.pop();
                current = { id: `${page.route}#${el.id}`, title, titles: trail.map(t => t.title), text: '' };
                docs.push(current);
                trail.push({ level: Number(level), title });
                continue;
            }
            walk(el);
            if (current && /^(P|LI|DIV|PRE|TR|DT|DD)$/.test(el.tagName)) current.text += ' ';
        }
    };
    walk(document.body);
    for (const doc of docs) doc.text = doc.text.replace(/\s+/g, ' ').trim();
    return docs;
}

export function searchIndex(pages: Page[]): string {
    const index = new MiniSearch<SearchDoc>(indexOptions);
    for (const page of pages) index.addAll(sections(page));
    return JSON.stringify(index);
}

const mdPath = (route: string) => `${route.endsWith('/') ? `${route}index` : route}.md`;

export function llmsTxt(pages: Page[]): string {
    return `# ${site.name}\n\n> ${site.description}\n\n## Docs\n\n${pages.map(p => `- [${p.title}](${mdPath(p.route)})${p.description ? `: ${p.description}` : ''}`).join('\n')}\n`;
}

export const llmsFullTxt = (pages: Page[]) => pages.map(p => `---\nurl: ${p.route}\n---\n\n${p.source.replace(/^<script setup>[\s\S]*?<\/script>\n/m, '').trim()}\n`).join('\n');

/** Each page's Markdown, next to its HTML, for llms.txt links. */
export const markdownFiles = (pages: Page[]) => new Map(pages.map(p => [mdPath(p.route).slice(1), p.source]));
