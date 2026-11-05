import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseHTML } from 'linkedom';
import { optionKey } from '../site/optionKey.ts';
import { docsRoot, slug } from './markdown.ts';
import { tools } from '../site/site.ts';
import markdownStyles from './markdown.module.css';
import styles from './specimens.module.css';

const anchor = markdownStyles.anchor!;

// Options become specimens: the description on the left, a spec card (type, default, flag, other tools) on the right.
// The card takes the facts from an option's "Type: …" list (the first list that opens with one); notes in the same list
// stay with the description. "Also in" links come from every tool's configuration page.

interface OptionRef {
    tool: string;
    name: string;
    id: string;
}

let registry: Map<string, OptionRef[]> | undefined;
function optionRegistry(): Map<string, OptionRef[]> {
    if (registry) return registry;
    registry = new Map();
    for (const tool of readdirSync(docsRoot)) {
        let source: string;
        try {
            source = readFileSync(join(docsRoot, tool, 'configuration.md'), 'utf8');
        } catch {
            continue;
        }
        for (const [, name] of source.matchAll(/^### `([^`]+)`\s*$/gm)) {
            const key = optionKey(name!);
            registry.set(key, [...(registry.get(key) ?? []), { tool, name: name!, id: slug(name!) }]);
        }
    }
    return registry;
}

const SPEC = /^(Type|Default|Required|Flags?|Manifest|Valid values|Values|Availability|Since|Env|Reference)\b(?::\s*|$)/;
const bare = (name: string) => name.toLowerCase().replace(/[^a-z0-9]/g, '');

export function specimens(html: string, section: string): string {
    if (!html.includes('<h3')) return html;
    const { document } = parseHTML(`<!doctype html><html><body><div id="root">${html}</div></body></html>`);
    const root = document.getElementById('root')!;
    for (const h3 of root.querySelectorAll('h3')) {
        const [first, ...rest] = h3.children;
        if (first?.tagName !== 'CODE' || h3.firstChild !== first || rest.length !== 1 || !rest[0]!.matches(`.${anchor}`)) continue;
        const article = document.createElement('article');
        article.className = styles.option!;
        article.setAttribute('aria-labelledby', h3.id);
        const desc = document.createElement('div');
        desc.className = styles.description!;
        h3.before(article);
        article.append(h3, desc);
        let spec: Element | undefined;
        while (article.nextElementSibling && !article.nextElementSibling.matches(`h2, h3, .${styles.option}`)) {
            const el = article.nextElementSibling;
            if (!spec && el.matches('ul') && SPEC.test(el.firstElementChild?.textContent ?? '')) spec = el;
            desc.append(el);
        }
        const dl = document.createElement('dl');
        dl.className = styles.spec!;
        for (const li of spec ? [...spec.children] : []) {
            const text = (li.textContent ?? '').trim();
            const key = SPEC.exec(text)?.[1];
            if (!key) continue;
            const row = document.createElement('div');
            // A bare "Required" carries no value of its own.
            const value = text === key ? 'yes' : li.innerHTML.replace(new RegExp(`^\\s*${key}\\b:\\s*`), '');
            row.innerHTML = `<dt>${key}</dt><dd>${value}</dd>`;
            dl.append(row);
            li.remove();
        }
        if (spec && !spec.children.length) spec.remove();
        const name = first.textContent ?? '';
        const others = (optionRegistry().get(optionKey(name)) ?? []).filter(o => o.tool !== section);
        if (others.length) {
            const row = document.createElement('div');
            row.className = styles.also!;
            row.innerHTML = `<dt>Also in</dt><dd>${others
                .map(
                    o =>
                        `<a href="/${o.tool}/configuration#${o.id}">${tools.find(t => t.key === o.tool)?.name ?? o.tool}${bare(o.name) === bare(name) ? '' : ` <code>${o.name}</code>`}</a>`,
                )
                .join('')}</dd>`;
            dl.append(row);
        }
        if (dl.children.length) article.append(dl);
        else article.classList.add(styles.bare!);
    }
    return root.innerHTML;
}
