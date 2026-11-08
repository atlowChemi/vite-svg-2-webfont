// The search dialog, loaded once the reader heads for it. The index is built with the site (`src/build/outputs.ts`) and
// fetched once a page; the browser's cache keeps it across pages. The last query survives page loads in sessionStorage.
// Each result names its section's path and heading. The detailed view, on unless the reader turns it off, adds the
// section itself, cut from its page's HTML, as VitePress's local search did.
import MiniSearch, { type SearchResult } from 'minisearch';
import prose from '../components/shell/Prose.module.css';
import { siteIcon, type SiteIcon } from '../site/siteIcons.ts';
import styles from './search.module.css';
import { indexOptions, searchOptions, type SearchDoc } from './searchOptions';

const QUERY_KEY = 'fonte-search-query';
const DETAILED_KEY = 'fonte-search-detailed';
const MAX_RESULTS = 16;
let dialog: HTMLDialogElement | undefined;
let index: Promise<MiniSearch<SearchDoc>> | undefined;
let indexLoaded = false;

/**
 * The index, fetched and loaded once a page. Started before the first query (when the reader heads for search, and
 * when the dialog opens), so typing doesn't wait for the download and MiniSearch's load. A failed load is tried again.
 */
export function loadIndex(): Promise<MiniSearch<SearchDoc>> {
    index ??= fetch('/search-index.json')
        .then(async r => {
            if (!r.ok) throw new Error(`The search index answered ${r.status}`);
            const loaded = MiniSearch.loadJSON<SearchDoc>(await r.text(), indexOptions);
            indexLoaded = true;
            return loaded;
        })
        .catch((error: unknown) => {
            index = undefined;
            throw error;
        });
    return index;
}

const escape = (s: string) => s.replace(/[&<>"]/g, c => `&#${c.charCodeAt(0)};`);
const icon = (name: SiteIcon, className = '') => `<i class="${siteIcon(name)} ${className}" aria-hidden="true"></i>`;

/** The text, escaped, with every matched term marked. */
function marked(text: string, terms: RegExp | undefined): string {
    if (!terms) return escape(text);
    let out = '';
    let last = 0;
    for (const match of text.matchAll(terms)) {
        out += `${escape(text.slice(last, match.index))}<mark>${escape(match[0])}</mark>`;
        last = match.index + match[0].length;
    }
    return out + escape(text.slice(last));
}

/** Marks every matched term in the text under `root`. */
function markIn(root: Element, terms: RegExp) {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const nodes: Text[] = [];
    while (walker.nextNode()) nodes.push(walker.currentNode as Text);
    for (const node of nodes) {
        const parts: (string | Node)[] = [];
        let last = 0;
        for (const match of node.data.matchAll(terms)) {
            parts.push(node.data.slice(last, match.index), Object.assign(document.createElement('mark'), { textContent: match[0] }));
            last = match.index + match[0].length;
        }
        if (parts.length) node.replaceWith(...parts, node.data.slice(last));
    }
}

const termPattern = (terms: string[]) =>
    terms.length
        ? new RegExp(
              terms
                  .toSorted((a, b) => b.length - a.length)
                  .map(t => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
                  .join('|'),
              'gi',
          )
        : undefined;

// Each result's page, fetched once: usually from the browser's cache, since the reader came from it or prefetched it.
const pages = new Map<string, Promise<Document | undefined>>();
function page(path: string): Promise<Document | undefined> {
    let doc = pages.get(path);
    if (!doc) {
        doc = fetch(path)
            .then(async r => (r.ok ? new DOMParser().parseFromString(await r.text(), 'text/html') : undefined))
            .catch(() => undefined);
        pages.set(path, doc);
    }
    return doc;
}

/**
 * A section's content: everything after its heading, up to the next heading. Its radios lose their names, so a copied
 * code group doesn't join the page's own group and uncheck its tab.
 */
async function section(id: string): Promise<DocumentFragment | undefined> {
    const [path, anchor] = id.split('#');
    let el: Element | null | undefined = anchor ? (await page(path!))?.getElementById(decodeURIComponent(anchor)) : undefined;
    if (!el) return undefined;
    const content = document.createDocumentFragment();
    while ((el = el.nextElementSibling) && !/^H[1-6]$/.test(el.tagName)) content.append(document.importNode(el, true));
    for (const input of content.querySelectorAll('input[name]')) input.removeAttribute('name');
    return content;
}

const storedDetailed = () => {
    try {
        return localStorage.getItem(DETAILED_KEY) !== 'false';
    } catch {
        return true;
    }
};

function build(): HTMLDialogElement {
    const el = document.createElement('dialog');
    el.className = styles.dialog!;
    el.setAttribute('aria-label', 'Search the docs');
    // A tap on the backdrop closes it; in browsers without `closedby`, the click below does.
    el.setAttribute('closedby', 'any');
    el.innerHTML = `<div class="${styles.bar}">${icon('search', styles.icon)}<input class="${styles.input}" type="search" placeholder="Search the docs" aria-label="Search the docs" aria-controls="search-results" autocomplete="off"><button type="button" class="${styles.tool}" aria-label="Show each section's content" title="Show each section's content" data-detailed>${icon('list-details')}</button><button type="button" class="${styles.tool}" aria-label="Clear the search" title="Clear the search" data-clear>${icon('backspace')}</button></div><ul id="search-results" class="${styles.results}" role="listbox"></ul><div class="${styles.keys}"><span><kbd>↑</kbd><kbd>↓</kbd> to navigate</span><span><kbd>↵</kbd> to select</span><span><kbd>esc</kbd> to close</span></div>`;
    const input = el.querySelector('input')!;
    const list = el.querySelector('ul')!;
    const detailedButton = el.querySelector<HTMLButtonElement>('[data-detailed]')!;
    const clearButton = el.querySelector<HTMLButtonElement>('[data-clear]')!;
    let active = 0;
    let shown: SearchResult[] = [];

    // The results themselves, not the links inside their excerpts.
    const options = () => [...list.querySelectorAll<HTMLAnchorElement>('a[role="option"]')];
    const mark = () => {
        const items = options();
        items.forEach((a, i) => a.setAttribute('aria-selected', String(i === active)));
        const current = items[active];
        if (current) input.setAttribute('aria-activedescendant', current.id);
        else input.removeAttribute('aria-activedescendant');
    };

    let detailed = storedDetailed();
    const showDetailed = () => {
        el.classList.toggle(styles.detailed!, detailed);
        detailedButton.setAttribute('aria-pressed', String(detailed));
    };
    showDetailed();

    /**
     * Fills each shown result's excerpt with its section, marked, and scrolled to the first match. Until its page
     * arrives, an excerpt holds a placeholder of its size, so the list doesn't jump; a section that can't be had leaves
     * the result as its heading alone.
     */
    async function excerpts(results: SearchResult[]) {
        const slots = [...list.querySelectorAll<HTMLElement>(`.${styles.excerpt}`)];
        for (const slot of slots) if (!slot.childNodes.length) slot.parentElement!.classList.add(styles.loading!);
        await Promise.all(
            results.map(async (r, i) => {
                const content = await section(r.id);
                const slot = slots[i];
                // A newer query has replaced the list, or the slot is already filled.
                if (!slot?.isConnected || slot.childNodes.length) return;
                slot.parentElement!.classList.remove(styles.loading!);
                if (!content) {
                    slot.parentElement!.classList.add(styles.missing!);
                    return;
                }
                slot.append(content);
                const terms = termPattern(r.terms);
                if (terms) markIn(slot, terms);
                const first = slot.querySelector('mark');
                const view = slot.parentElement!;
                if (first) view.scrollTop = first.getBoundingClientRect().top - view.getBoundingClientRect().top - view.clientHeight / 3;
            }),
        );
    }

    async function run() {
        const q = input.value.trim();
        clearButton.disabled = !input.value;
        try {
            sessionStorage.setItem(QUERY_KEY, input.value);
        } catch {}
        if (!q) {
            shown = [];
            list.innerHTML = '';
            mark();
            return;
        }
        // The first query waits for the index, so say so.
        if (!indexLoaded) {
            list.setAttribute('aria-busy', 'true');
            list.innerHTML = `<li class="${styles.status}">Loading the search index…</li>`;
        }
        let search: MiniSearch<SearchDoc>;
        try {
            search = await loadIndex();
        } catch {
            if (input.value.trim() !== q) return;
            shown = [];
            list.innerHTML = `<li class="${styles.status}">Search couldn't load. <button type="button" class="${styles.retry}" data-retry>Try again</button></li>`;
            mark();
            return;
        } finally {
            list.removeAttribute('aria-busy');
        }
        const results = search.search(q, searchOptions).slice(0, MAX_RESULTS);
        if (input.value.trim() !== q) return;
        active = 0;
        shown = results;
        list.innerHTML = results.length
            ? results
                  .map((r, i) => {
                      const trail = (r.titles as string[]).map(t => `<span>${escape(t)}</span>${icon('chevron-down', styles.chevron)}`).join('');
                      return `<li><a class="${styles.result}" role="option" id="search-result-${i}" href="${escape(r.id)}"><span class="${styles.titles}"><span class="${styles.hash}" aria-hidden="true">#</span>${trail}<span class="${styles.title}">${marked(r.title as string, termPattern(r.terms))}</span></span><div class="${styles.view}"><div class="${styles.excerpt} ${prose.prose}" inert></div></div></a></li>`;
                  })
                  .join('')
            : `<li class="${styles.empty}">No results for “${escape(q)}”.</li>`;
        mark();
        if (detailed) await excerpts(results);
    }

    input.addEventListener('input', () => void run());
    input.addEventListener('keydown', e => {
        const items = options();
        if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
            e.preventDefault();
            active = (active + (e.key === 'ArrowDown' ? 1 : -1) + items.length) % Math.max(items.length, 1);
            mark();
            items[active]?.scrollIntoView({ block: 'nearest' });
            return;
        }
        const item = items[active];
        if (e.key === 'Enter' && item) {
            e.preventDefault();
            item.click();
        }
    });
    // The pointer selects what it rests on, so Enter and the arrows carry on from there.
    list.addEventListener('pointermove', e => {
        const i = options().indexOf((e.target as HTMLElement).closest<HTMLAnchorElement>('a[role="option"]')!);
        if (i >= 0 && i !== active) {
            active = i;
            mark();
        }
    });
    // Following a result on this page only moves the hash, so close the dialog too.
    list.addEventListener('click', e => {
        const target = e.target as HTMLElement;
        if (target.closest('[data-retry]')) {
            void run();
            input.focus();
        } else if (target.closest('a')) el.close();
    });
    detailedButton.addEventListener('click', () => {
        detailed = !detailed;
        showDetailed();
        try {
            localStorage.setItem(DETAILED_KEY, String(detailed));
        } catch {}
        if (detailed) void excerpts(shown);
        input.focus();
    });
    clearButton.addEventListener('click', () => {
        input.value = '';
        void run();
        input.focus();
    });
    if (!('closedBy' in HTMLDialogElement.prototype)) {
        // A tap on the backdrop lands on the dialog itself.
        el.addEventListener('click', e => {
            if (e.target === el) el.close();
        });
    }
    try {
        input.value = sessionStorage.getItem(QUERY_KEY) ?? '';
    } catch {}
    clearButton.disabled = !input.value;
    document.body.append(el);
    return el;
}

export function open() {
    loadIndex().catch(() => {});
    dialog ??= build();
    dialog.showModal();
    const input = dialog.querySelector('input')!;
    input.select();
    if (input.value && !dialog.querySelector('li')) input.dispatchEvent(new Event('input'));
}
