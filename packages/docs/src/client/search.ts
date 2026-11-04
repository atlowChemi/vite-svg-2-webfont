// The search dialog, loaded on first use. The index is built with the site (`src/build/outputs.ts`) and fetched once; the browser's
// cache keeps it across pages. The last query survives page loads in sessionStorage.
import MiniSearch from 'minisearch';
import { indexOptions, searchOptions, type SearchDoc } from './searchOptions';

const QUERY_KEY = 'fonte-search-query';
let dialog: HTMLDialogElement | undefined;
let index: Promise<MiniSearch<SearchDoc>> | undefined;

const escape = (s: string) => s.replace(/[&<>"]/g, c => `&#${c.charCodeAt(0)};`);

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

/** About 160 characters of the section's text around its first match. */
function excerpt(text: string, terms: RegExp | undefined): string {
    const at = terms ? Math.max(0, text.search(terms)) : 0;
    const start = at > 60 ? text.lastIndexOf(' ', at - 50) + 1 : 0;
    const end = Math.min(text.length, start + 160);
    const cut = end < text.length ? text.lastIndexOf(' ', end) : end;
    return `${start ? '…' : ''}${marked(text.slice(start, cut > start ? cut : end), terms)}${end < text.length ? '…' : ''}`;
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

function build(): HTMLDialogElement {
    const el = document.createElement('dialog');
    el.className = 'doc-search-dialog';
    el.setAttribute('aria-label', 'Search the docs');
    el.innerHTML = `<input type="search" placeholder="Search the docs" aria-label="Search the docs" aria-controls="doc-search-results" autocomplete="off"><ul id="doc-search-results" role="listbox"></ul><p class="hint dim">↑↓ to move, Enter to open, Esc to close</p>`;
    const input = el.querySelector('input')!;
    const list = el.querySelector('ul')!;
    let active = 0;

    const options = () => [...list.querySelectorAll<HTMLAnchorElement>('a')];
    const mark = () => options().forEach((a, i) => a.setAttribute('aria-selected', String(i === active)));
    async function run() {
        const q = input.value.trim();
        try {
            sessionStorage.setItem(QUERY_KEY, input.value);
        } catch {}
        if (!q) {
            list.innerHTML = '';
            return;
        }
        const results = (await (index ??= fetch('/search-index.json').then(async r => MiniSearch.loadJSON<SearchDoc>(await r.text(), indexOptions))))
            .search(q, searchOptions)
            .slice(0, 12);
        if (input.value.trim() !== q) return;
        active = 0;
        list.innerHTML = results.length
            ? results
                  .map(r => {
                      const terms = termPattern(r.terms);
                      const text = r.text as string;
                      return `<li><a role="option" href="${escape(r.id)}"><span class="trail">${[...(r.titles as string[])].map(escape).join(' › ')}</span><strong>${marked(r.title as string, terms)}</strong>${text ? `<span class="excerpt">${excerpt(text, terms)}</span>` : ''}</a></li>`;
                  })
                  .join('')
            : `<li class="dim">No results for “${escape(q)}”.</li>`;
        mark();
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
    // Following a result on this page only moves the hash, so close the dialog too.
    list.addEventListener('click', e => {
        if ((e.target as HTMLElement).closest('a')) el.close();
    });
    el.addEventListener('click', e => {
        if (e.target === el) el.close();
    });
    try {
        input.value = sessionStorage.getItem(QUERY_KEY) ?? '';
    } catch {}
    document.body.append(el);
    return el;
}

export function open() {
    dialog ??= build();
    dialog.showModal();
    const input = dialog.querySelector('input')!;
    input.select();
    if (input.value && !dialog.querySelector('li')) input.dispatchEvent(new Event('input'));
}
