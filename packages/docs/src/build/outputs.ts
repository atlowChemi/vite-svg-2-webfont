import { parseHTML } from 'linkedom';
import MiniSearch from 'minisearch';
import { indexOptions, type SearchDoc } from '../client/searchOptions.ts';
import { normalize, sections as navSections, site } from '../site/site.ts';
import { extractBlocks } from './blocks.ts';
import type { Page } from './render.tsx';

// Everything derived from the rendered pages in the same pass: the search index, llms.txt / llms-full.txt and each
// page's Markdown.

/**
 * A page's rendered HTML, parsed once in an element of its own: specimens rewrite it, and the search sections are read
 * from it.
 */
export function parseRendered(html: string): Element {
    return parseHTML(`<!doctype html><html><body><div>${html}</div></body></html>`).document.body.firstElementChild!;
}

const skip = new Set(['BUTTON', 'SCRIPT', 'STYLE', 'svg', 'SVG']);
/** A page's search sections, one per heading, read from its parsed HTML. */
export function searchSections(root: Element, route: string): SearchDoc[] {
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
            const level = el.tagName.length === 2 && el.tagName[0] === 'H' && ['1', '2', '3', '4', '5', '6'].includes(el.tagName[1]!) ? Number(el.tagName[1]) : 0;
            if (level && el.id) {
                const title = (el.textContent ?? '').replace(/​/g, '').trim();
                while (trail.length && trail.at(-1)!.level >= level) trail.pop();
                current = { id: `${route}#${el.id}`, title, titles: trail.map(t => t.title), text: '' };
                docs.push(current);
                trail.push({ level, title });
                continue;
            }
            walk(el);
            // Block elements end a run of text, and so does an element right next to another, such as a list of links.
            if (current && (/^(P|LI|DIV|PRE|TR|DT|DD)$/.test(el.tagName) || el.nextSibling?.nodeType === 1)) current.text += ' ';
        }
    };
    walk(root);
    for (const doc of docs) doc.text = doc.text.replace(/\s+/g, ' ').trim();
    return docs;
}

export function searchIndex(pages: Page[]): string {
    const index = new MiniSearch<SearchDoc>(indexOptions);
    index.addAll(pages.flatMap(p => p.sections));
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

// The URL is the page's Markdown copy, which llms.txt links too.
const frontmatter = (page: Page) =>
    `---\ntitle: ${JSON.stringify(page.title)}\n${page.description ? `description: ${JSON.stringify(page.description)}\n` : ''}url: ${mdPath(page.route)}\n---\n\n`;

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

/** The page's components, and those of them that only show something, such as a live font preview. */
export interface MarkdownBlocks {
    names: ReadonlySet<string>;
    /** Dropped from the Markdown, slots and all: their content is HTML for the eye, which would read as noise. */
    visual: ReadonlySet<string>;
}

/**
 * A page's Markdown for readers that aren't browsers: components give way to the Markdown in their slots (an example's
 * config, for one), visual-only components go, and HTML tags outside code are dropped, keeping their text.
 */
export function plainMarkdown(source: string, components: MarkdownBlocks): string {
    const { source: text, blocks } = extractBlocks(source, new Set(components.names), i => `<!--fonte-block:${i}-->`);
    let out = text;
    blocks.forEach((block, i) => {
        const slots = components.visual.has(block.name)
            ? []
            : Object.values(block.slots)
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

export function pageMarkdown(page: Page, components: MarkdownBlocks): string {
    return `${frontmatter(page)}${plainMarkdown(page.source, components)}\n`;
}

// A `---` line closes each page before the next one's frontmatter, so a reader splitting the file finds every boundary
// even where a page ends in a rule of its own, as the VitePress build's file did.
export const llmsFullTxt = (pages: Page[], components: MarkdownBlocks) =>
    ordered(pages)
        .flatMap(g => g.pages)
        .map(page => pageMarkdown(page, components))
        .join('\n---\n\n');

/** Each page's Markdown, next to its HTML, for llms.txt links. */
export const markdownFiles = (pages: Page[], components: MarkdownBlocks) => new Map(pages.map(p => [mdPath(p.route).slice(1), pageMarkdown(p, components)]));
