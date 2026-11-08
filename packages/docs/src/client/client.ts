// Everything the shell does in the browser. Pages arrive fully rendered; this only follows the reader and answers clicks.
// It finds the shell's parts by their `data-*` attributes, and toggles the state classes their CSS modules define.
import code from '../build/highlight.module.css';
import rail from '../components/shell/Rail.module.css';
import sheets from '../components/shell/Sheets.module.css';
import { isDark, systemDark } from './theme.ts';

const root = document.documentElement;
const reducedMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

// Theme. The head script already applied a saved choice before the first paint, under VitePress's key so it carries
// over; with none, CSS follows the system. A choice that matches the system is stored as `auto`, as VitePress did, and
// clears the class, so the page follows the system from then on.
const THEME_KEY = 'vitepress-theme-appearance';
function toggleTheme() {
    const dark = !isDark();
    const auto = dark === systemDark.matches;
    root.classList.toggle('dark', dark && !auto);
    root.classList.toggle('light', !dark && !auto);
    try {
        localStorage.setItem(THEME_KEY, auto ? 'auto' : dark ? 'dark' : 'light');
    } catch {}
}

// The search shortcut, as this platform spells it.
if (!/Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent)) {
    for (const kbd of document.querySelectorAll('[data-search] kbd')) kbd.textContent = 'Ctrl K';
}

// The rail's index follows the reader: the current entry is the last heading above the bars, so it stays right after
// any jump (links, search, "Also in").
const index = document.querySelector<HTMLElement>('[data-index]');
const railEl = index?.closest<HTMLElement>('[data-rail]');
const headings = [...document.querySelectorAll<HTMLElement>('[data-content] :is(h2, h3)[id]')];
const sections = headings.filter(h => h.tagName === 'H2');
const pill = document.querySelector<HTMLElement>('[data-pill]');
const linksTo = (id: string) => document.querySelectorAll<HTMLAnchorElement>(`[data-rail] a[href="#${CSS.escape(id)}"], [data-section][href="#${CSS.escape(id)}"]`);
const label = (h: HTMLElement) => (h.textContent ?? '').replace(/​/g, '').trim();

function revealActive() {
    const link = railEl?.querySelector<HTMLElement>('a[aria-current="location"]');
    if (!railEl || !link || railEl.offsetParent === null) return;
    const { top, bottom } = link.getBoundingClientRect();
    const view = railEl.getBoundingClientRect();
    if (top < view.top + 40) railEl.scrollTop -= view.top + 40 - top;
    else if (bottom > view.bottom - 40) railEl.scrollTop += bottom - view.bottom + 40;
}

// Track from the lowest bar: the strip, or the top bar on phones, where the hidden strip's box is all zeros.
const barBottom = (selector: string) => document.querySelector(selector)?.getBoundingClientRect().bottom ?? 0;
let frame = 0;
let shown: string | undefined;
function track() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
        const top = Math.max(barBottom('[data-page-strip]'), barBottom('[data-top-bar]')) + 24;
        const atEnd = innerHeight + scrollY >= root.scrollHeight - 2;
        let found = atEnd ? headings.at(-1) : undefined;
        if (!found) for (const h of headings) if (h.getBoundingClientRect().top <= top) found = h;
        found ??= headings[0];
        if (!found || found.id === shown) return;
        shown = found.id;
        const h2 = found.tagName === 'H2' ? found : sections.findLast(h => h.compareDocumentPosition(found) & Node.DOCUMENT_POSITION_FOLLOWING);
        for (const a of document.querySelectorAll('[data-rail] a[aria-current], [data-section][aria-current]')) a.removeAttribute('aria-current');
        for (const a of document.querySelectorAll(`[data-rail] a.${rail.within}`)) a.classList.remove(rail.within!);
        for (const a of linksTo(found.id)) a.setAttribute('aria-current', 'location');
        if (h2) {
            for (const a of linksTo(h2.id)) {
                if (a.matches('[data-rail] a')) a.classList.add(rail.within!);
                else a.setAttribute('aria-current', 'location');
            }
            if (pill) {
                pill.querySelector('[data-pill-number]')!.textContent = String(sections.indexOf(h2) + 1).padStart(2, '0');
                pill.querySelector('[data-pill-text]')!.textContent = label(h2);
            }
        }
        revealActive();
    });
}

// The filter searches every section's entries, in the rail and in the sections sheet. While it has text, CSS shows every
// section's entries rather than only the current one's.
function filterRail(q: string) {
    if (!index) return;
    let any = false;
    for (const li of index.querySelectorAll<HTMLElement>(':scope > ol > li')) {
        const children = [...li.querySelectorAll<HTMLElement>('ol > li')];
        for (const c of children) c.hidden = !!q && !c.textContent.toLowerCase().includes(q);
        const list = li.querySelector<HTMLElement>('ol');
        const matches = children.some(c => !c.hidden);
        if (list) list.hidden = !!q && !matches;
        li.hidden = !!q && !matches && !li.firstElementChild!.textContent.toLowerCase().includes(q);
        any ||= !li.hidden;
    }
    index.querySelector<HTMLElement>('[data-no-match]')!.hidden = any;
}

function filterSheet(sheet: HTMLElement, q: string) {
    let section: HTMLElement | undefined;
    let sectionMatches = false;
    const entries = [...sheet.querySelectorAll<HTMLElement>('[data-section]')];
    // A section stays when it or one of its entries matches.
    const keep = new Set<HTMLElement>();
    for (const a of entries) {
        if (a.dataset.section !== 'child') {
            section = a;
            sectionMatches = !q || a.textContent.toLowerCase().includes(q);
            if (sectionMatches) keep.add(a);
            continue;
        }
        if (!q || a.textContent.toLowerCase().includes(q)) {
            keep.add(a);
            if (section) keep.add(section);
        }
    }
    for (const a of entries) a.hidden = !keep.has(a);
}

for (const input of document.querySelectorAll<HTMLInputElement>('[data-filter]')) {
    input.addEventListener('input', () => {
        const q = input.value.trim().toLowerCase();
        const sheet = input.closest<HTMLElement>('dialog');
        if (sheet) filterSheet(sheet, q);
        else filterRail(q);
    });
}

// Phones: the section pill steps aside while you scroll down, and comes back when you scroll up.
let lastY = scrollY;
addEventListener(
    'scroll',
    () => {
        track();
        pill?.classList.toggle(sheets.away!, scrollY > lastY && scrollY > 200);
        lastY = scrollY;
    },
    { passive: true },
);
addEventListener('resize', track);
addEventListener('hashchange', track);
track();
document.querySelector('[data-page-strip] [aria-current="page"]')?.scrollIntoView({ inline: 'center', block: 'nearest' });

// A deep link lands before fonts and late blocks settle, which pushes the heading down; land again once they have,
// unless the reader has already moved.
if (location.hash) {
    let moved = false;
    for (const type of ['wheel', 'touchstart', 'keydown']) addEventListener(type, () => (moved = true), { once: true, passive: true });
    void document.fonts.ready.then(() =>
        setTimeout(() => {
            if (!moved) document.getElementById(decodeURIComponent(location.hash.slice(1)))?.scrollIntoView();
        }, 300),
    );
}

// The sheets open from their buttons' `commandfor`, and close on a backdrop tap through `closedby`. Browsers without
// those get both here.
const nativeCommands = 'command' in HTMLButtonElement.prototype;
const nativeClosedBy = 'closedBy' in HTMLDialogElement.prototype;
function openSheet(button: HTMLElement) {
    (document.getElementById(button.dataset.sheet!) as HTMLDialogElement | null)?.showModal();
}
// An opened sheet takes focus itself, as a page would: the browser focuses its first control, and focusing the filter
// would open the phone's keyboard. (`autofocus` on the dialog should do this, but Chromium still focuses the control.)
for (const button of document.querySelectorAll<HTMLElement>('[data-sheet]')) {
    const dialog = document.getElementById(button.dataset.sheet!);
    dialog?.addEventListener('toggle', e => e.newState === 'open' && dialog.focus());
}

// Copying a code block copies what it shows, and shell commands without their prompts.
async function copy(button: HTMLElement) {
    const block = button.parentElement;
    const pre = block?.querySelector('pre');
    if (!block || !pre) return;
    let text = pre.textContent ?? '';
    if (/^(?:shellscript|shell|bash|sh|zsh)$/.test(block.dataset.lang ?? '')) text = text.replace(/^ *[$>] /gm, '').trim();
    await navigator.clipboard.writeText(text);
    button.classList.add(code.copied!);
    setTimeout(() => button.classList.remove(code.copied!), 2000);
}

let search: Promise<typeof import('./search.ts')> | undefined;
const searchModule = () => (search ??= import('./search.ts'));
const openSearch = () => void searchModule().then(m => m.open());
// A pointer on the search button, or focus reaching it, means a search is likely: load the dialog and its index then,
// so the first query doesn't wait for them. The shortcut keys open the dialog, which starts the same load.
const prepareSearch = (e: Event) => {
    if (!(e.target as Element).closest?.('[data-search]')) return;
    searchModule()
        .then(m => m.loadIndex())
        .catch(() => {});
};
document.addEventListener('pointerover', prepareSearch);
document.addEventListener('focusin', prepareSearch);

document.addEventListener('click', e => {
    const target = e.target as HTMLElement;
    if (target.closest('[data-theme-toggle]')) return toggleTheme();
    const sheetButton = !nativeCommands && target.closest<HTMLElement>('[data-sheet]');
    if (sheetButton) return openSheet(sheetButton);
    // A tap on the backdrop lands on the dialog itself.
    if (!nativeClosedBy && target instanceof HTMLDialogElement) return target.close();
    const section = target.closest<HTMLAnchorElement>('[data-section]');
    if (section) {
        e.preventDefault();
        section.closest('dialog')!.close();
        const heading = document.getElementById(decodeURIComponent(section.hash.slice(1)));
        if (!heading) return;
        history.replaceState(null, '', section.hash);
        // Closing hands focus back to the pill, which cancels a scroll started in the same frame; move on the next one.
        requestAnimationFrame(() => {
            heading.scrollIntoView({ behavior: reducedMotion() ? 'auto' : 'smooth' });
            heading.setAttribute('tabindex', '-1');
            heading.focus({ preventScroll: true });
        });
        return;
    }
    const copyButton = target.closest<HTMLElement>(`.${code.copy}`);
    if (copyButton) return void copy(copyButton);
    if (target.closest('[data-search]')) openSearch();
});

addEventListener('keydown', e => {
    if ((e.key === 'k' && (e.metaKey || e.ctrlKey)) || (e.key === '/' && !(e.target as HTMLElement).matches('input, textarea, [contenteditable]'))) {
        e.preventDefault();
        openSearch();
    }
});

// Browsers without speculation rules prefetch a page once the pointer rests on its link.
if (!HTMLScriptElement.supports?.('speculationrules')) {
    const fetched = new Set([location.pathname]);
    let timer = 0;
    document.addEventListener('pointerover', e => {
        const a = (e.target as HTMLElement).closest('a');
        clearTimeout(timer);
        if (!a || a.origin !== location.origin || fetched.has(a.pathname)) return;
        timer = window.setTimeout(() => {
            fetched.add(a.pathname);
            document.head.append(Object.assign(document.createElement('link'), { rel: 'prefetch', href: a.pathname }));
        }, 80);
    });
}
