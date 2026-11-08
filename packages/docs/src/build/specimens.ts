import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { optionKey } from '../site/optionKey.ts';
import { pagesRoot, slug } from './markdown.ts';
import { tools } from '../site/site.ts';
import markdownStyles from './markdown.module.css';
import styles from './specimens.module.css';

const anchor = markdownStyles.anchor!;

// Options become specimens: a description on the left and a spec card on the right.
// The card extracts metadata from the first list beginning with a recognized field.
// "Also in" links reference equivalent options across tool configuration pages.

interface OptionRef {
    tool: string;
    name: string;
    id: string;
}

// Cache tool display names to avoid repeated linear searches.
const toolNames: ReadonlyMap<string, string> = new Map(tools.map(tool => [tool.key, tool.name]));

let registry: Promise<Map<string, OptionRef[]>> | undefined;

/** Lazily indexes configuration options across tools for cross-references. */
function optionRegistry(): Promise<Map<string, OptionRef[]>> {
    return (registry ??= buildOptionRegistry());
}

/** Reads configuration pages concurrently and indexes their options. */
async function buildOptionRegistry(): Promise<Map<string, OptionRef[]>> {
    const directories = await readdir(pagesRoot);
    const result = new Map<string, OptionRef[]>();

    // Read all configuration pages concurrently.
    const sources = await Promise.all(
        directories.map(async tool => {
            try {
                return await readFile(join(pagesRoot, tool, 'configuration.md'), 'utf8');
            } catch {
                // Not every tool has a configuration page.
                return undefined;
            }
        }),
    );

    // Process results in directory order, regardless of read completion order.
    for (let i = 0; i < directories.length; i++) {
        const source = sources[i];
        if (source === undefined) continue;

        const tool = directories[i]!;

        for (const [, name] of source.matchAll(/^### `([^`]+)`\s*$/gm)) {
            const key = optionKey(name!);
            const existing = result.get(key);
            const ref = { tool, name: name!, id: slug(name!) };

            if (existing) existing.push(ref);
            else result.set(key, [ref]);
        }
    }

    return result;
}

// Recognized specification fields at the beginning of list items.
const SPEC = /^(Type|Default|Required|Flags?|Manifest|Valid values|Values|Availability|Since|Env|Reference)\b(?::\s*|$)/;
const bare = (name: string) => name.toLowerCase().replace(/[^a-z0-9]/g, '');

/**
 * Rewrites option sections into specimen cards in place.
 *
 * Each card contains the original heading and description, extracted
 * specification fields, and links to equivalent options in other tools.
 *
 * Returns whether any sections were transformed.
 */
export async function specimens(root: Element, section: string): Promise<boolean> {
    const allOptions = await optionRegistry();
    const document = root.ownerDocument;
    let changed = false;

    for (const h3 of root.querySelectorAll('h3')) {
        const first = h3.firstElementChild;

        // Only transform headings containing a code element followed by an anchor.
        if (first?.tagName !== 'CODE' || h3.firstChild !== first || h3.childElementCount !== 2 || !h3.lastElementChild?.classList.contains(anchor)) continue;

        changed = true;

        // Wrap the heading and following content in a specimen article.
        const article = document.createElement('article');
        article.className = styles.option!;
        article.setAttribute('aria-labelledby', h3.id);

        const desc = document.createElement('div');
        desc.className = styles.description!;

        h3.before(article);
        article.append(h3, desc);

        // Move following elements into the description until the next section.
        // The first list beginning with a recognized field contains spec metadata.
        let spec: Element | undefined;

        while (article.nextElementSibling) {
            const el = article.nextElementSibling;

            if (el.matches(`h2, h3, .${styles.option}`)) break;

            if (!spec && el.matches('ul') && SPEC.test(el.firstElementChild?.textContent ?? '')) spec = el;

            desc.append(el);
        }

        const dl = document.createElement('dl');
        dl.className = styles.spec!;

        if (spec) {
            // Re-reads the list each pass: a removed item's successor takes its index, so only a kept item advances.
            for (let i = 0; i < spec.children.length;) {
                const li = spec.children[i]!;
                const text = (li.textContent ?? '').trim();
                const key = SPEC.exec(text)?.[1];

                if (!key) {
                    i++;
                    continue;
                }

                // Built from nodes, not markup: the key is the item's text, and the value keeps its inline formatting by
                // moving the item's own nodes, so no text is ever read back as HTML.
                const row = document.createElement('div');
                const dt = document.createElement('dt');
                dt.textContent = key;
                const dd = document.createElement('dd');

                // A standalone "Required" means the option is required.
                if (text === key) dd.textContent = 'yes';
                else {
                    dd.append(...li.childNodes);
                    // The key and its colon open the item's first text.
                    const lead = dd.firstChild;
                    if (lead?.nodeType === 3) lead.textContent = (lead.textContent ?? '').replace(new RegExp(`^\\s*${key}\\b:\\s*`), '');
                }

                row.append(dt, dd);
                dl.append(row);

                li.remove();
            }

            // Remove the original list if no descriptive notes remain.
            if (!spec.children.length) spec.remove();
        }

        // Find equivalent options in other tools.
        const name = first.textContent ?? '';
        const others = (allOptions.get(optionKey(name)) ?? []).filter(ref => ref.tool !== section);

        if (others.length) {
            const row = document.createElement('div');
            row.className = styles.also!;

            // Names and generated HTML come from trusted documentation sources.
            row.innerHTML = `<dt>Also in</dt><dd>${others
                .map(
                    ref =>
                        `<a href="/${ref.tool}/configuration#${ref.id}">${toolNames.get(ref.tool) ?? ref.tool}${bare(ref.name) === bare(name) ? '' : ` <code>${ref.name}</code>`}</a>`,
                )
                .join('')}</dd>`;

            dl.append(row);
        }

        // Omit the spec card entirely when no metadata or references exist.
        if (dl.children.length) article.append(dl);
        else article.classList.add(styles.bare!);
    }

    return changed;
}
