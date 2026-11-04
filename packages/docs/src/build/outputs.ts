import { parseHTML } from 'linkedom';
import MiniSearch from 'minisearch';
import { indexOptions, type SearchDoc } from '../client/searchOptions.ts';
import { normalize, sections as navSections, site } from '../site/site.ts';
import { extractBlocks } from './blocks.ts';
import type { Page } from './render.tsx';

// Everything derived from the rendered pages in the same pass: the search index, llms.txt / llms-full.txt and each
// page's Markdown.

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
            // Every heading level starts a section, as in VitePress's index.
            const level = /^H([1-6])$/.exec(el.tagName)?.[1];
            if (level && el.id) {
                const title = (el.textContent ?? '').replace(/​/g, '').trim();
                while (trail.length && trail.at(-1)!.level >= Number(level)) trail.pop();
                current = { id: `${page.route}#${el.id}`, title, titles: trail.map(t => t.title), text: '' };
                docs.push(current);
                trail.push({ level: Number(level), title });
                continue;
            }
            walk(el);
            // Block elements end a run of text, and so does an element right next to another, such as a list of links.
            if (current && (/^(P|LI|DIV|PRE|TR|DT|DD)$/.test(el.tagName) || el.nextSibling?.nodeType === 1)) current.text += ' ';
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

/** The URL of a page's Markdown copy: `/guide/` is `/guide/index.md`. */
export const mdPath = (route: string) => `${route.endsWith('/') ? `${route}index` : route}.md`;

/** The pages in sidebar order, each group under its heading; pages outside the navigation come last. */
function ordered(pages: Page[]): { heading: string; pages: Page[] }[] {
    const byRoute = new Map(pages.map(p => [normalize(p.route), p]));
    const listed = new Set<Page>();
    const groups = Object.values(navSections).flatMap(shelves =>
        shelves.map(group => ({
            heading: group.text,
            pages: group.items.flatMap(item => {
                const page = byRoute.get(normalize(item.link));
                if (!page || listed.has(page)) return [];
                listed.add(page);
                return [page];
            }),
        })),
    );
    return [...groups, { heading: 'More', pages: pages.filter(p => !listed.has(p)) }].filter(g => g.pages.length);
}

export function llmsTxt(pages: Page[]): string {
    const toc = ordered(pages)
        .map(({ heading, pages: listed }) => `### ${heading}\n\n${listed.map(p => `- [${p.title}](${mdPath(p.route)})${p.description ? `: ${p.description}` : ''}`).join('\n')}`)
        .join('\n\n');
    return `# ${site.name}\n\n> ${site.description}\n\n## Table of Contents\n\n${toc}\n`;
}

const frontmatter = (page: Page) =>
    `---\ntitle: ${JSON.stringify(page.title)}\n${page.description ? `description: ${JSON.stringify(page.description)}\n` : ''}url: ${page.route}\n---\n\n`;

/** Removes HTML tags and comments, repeating until none are left, so removing one can't join the pieces of another. */
function stripTags(text: string): string {
    let previous;
    let out = text;
    do {
        previous = out;
        out = out.replace(/<\/?[A-Za-z][\w-]*(?:\s[^<>]*)?\/?>|<!--[\s\S]*?-->/g, '');
    } while (out !== previous);
    return out;
}

/**
 * A page's Markdown for readers that aren't browsers: components give way to the Markdown in their slots (an example's
 * config, for one), and HTML tags outside code are dropped, keeping their text.
 */
export function plainMarkdown(source: string, components: Set<string>): string {
    const { source: text, blocks } = extractBlocks(source, components, i => `<!--fonte-block:${i}-->`);
    let out = text;
    blocks.forEach((block, i) => {
        const slots = Object.values(block.slots)
            .map(slot => plainMarkdown(slot, components).trim())
            .filter(Boolean);
        out = out.replace(`<!--fonte-block:${i}-->`, () => slots.join('\n\n'));
    });
    let fence: string | undefined;
    return out
        .split('\n')
        .map(line => {
            const marker = /^\s*(`{3,}|~{3,})/.exec(line)?.[1];
            if (fence) {
                if (marker && marker[0] === fence[0] && marker.length >= fence.length) fence = undefined;
                return line;
            }
            if (marker) {
                fence = marker;
                return line;
            }
            // A heading's `{#id}` only sets its HTML id. Inline code keeps its tags: only the parts outside backticks are prose.
            return line
                .replace(/^(#{1,6} .*?)\s*\{#[\w-]+\}\s*$/, '$1')
                .split(/(`+[^`]*`+)/)
                .map((part, n) => (n % 2 ? part : stripTags(part)))
                .join('');
        })
        .join('\n')
        .replace(/\n[ \t]+\n/g, '\n\n')
        .replace(/\n{3,}/g, '\n\n')
        .trim();
}

export const llmsFullTxt = (pages: Page[], components: Set<string>) =>
    ordered(pages)
        .flatMap(g => g.pages)
        .map(p => `${frontmatter(p)}${plainMarkdown(p.source, components)}\n`)
        .join('\n');

/** Each page's Markdown, next to its HTML, for llms.txt links. */
export const markdownFiles = (pages: Page[], components: Set<string>) =>
    new Map(pages.map(p => [mdPath(p.route).slice(1), `${frontmatter(p)}${plainMarkdown(p.source, components)}\n`]));
