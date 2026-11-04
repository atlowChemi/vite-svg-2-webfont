// Everything the shell does in the browser. Pages arrive fully rendered; this only follows the reader and answers clicks.

const root = document.documentElement;
const reducedMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

// Theme. The head script already applied the saved choice before the first paint; VitePress's key, so it carries over.
function themeLabels() {
    const dark = root.classList.contains('dark');
    for (const button of document.querySelectorAll<HTMLElement>('[data-theme-toggle]')) {
        const label = button.querySelector('.label');
        if (label) label.textContent = dark ? 'Use light' : 'Use dark';
        else button.title = button.ariaLabel = dark ? 'Switch to light theme' : 'Switch to dark theme';
    }
}
themeLabels();

const THEME_KEY = 'vitepress-theme-appearance';
const systemDark = matchMedia('(prefers-color-scheme: dark)');
const storedTheme = () => {
    try {
        return localStorage.getItem(THEME_KEY) ?? 'auto';
    } catch {
        return 'auto';
    }
};

// A choice that matches the system is stored as `auto`, as VitePress did, so the page follows the system from then on.
function toggleTheme() {
    const dark = root.classList.toggle('dark');
    try {
        localStorage.setItem(THEME_KEY, dark === systemDark.matches ? 'auto' : dark ? 'dark' : 'light');
    } catch {}
    themeLabels();
}

// On `auto`, an open page follows the system switching between light and dark.
systemDark.addEventListener('change', () => {
    if (storedTheme() !== 'auto') return;
    root.classList.toggle('dark', systemDark.matches);
    themeLabels();
});

// The search shortcut, as this platform spells it.
if (!/Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent)) {
    for (const kbd of document.querySelectorAll('.doc-search-button kbd')) kbd.textContent = 'Ctrl K';
}

// The rail's index follows the reader: the current entry is the last heading above the bars, so it stays right after
// any jump (links, search, "Also in").
const index = document.querySelector<HTMLElement>('[data-index]');
const rail = index?.closest<HTMLElement>('.doc-rail');
const headings = [...document.querySelectorAll<HTMLElement>('.doc-main .vp-doc :is(h2, h3)[id]')];
const sections = headings.filter(h => h.tagName === 'H2');
const pill = document.querySelector<HTMLElement>('.doc-pill');
const linksTo = (id: string) => document.querySelectorAll<HTMLAnchorElement>(`.doc-rail a[href="#${CSS.escape(id)}"], .doc-sheet a[href="#${CSS.escape(id)}"]`);
const label = (h: HTMLElement) => (h.textContent ?? '').replace(/​/g, '').trim();
let filtering = false;

function revealActive() {
    const link = rail?.querySelector<HTMLElement>('a[aria-current="location"]');
    if (!rail || !link || rail.offsetParent === null) return;
    const { top, bottom } = link.getBoundingClientRect();
    const view = rail.getBoundingClientRect();
    if (top < view.top + 40) rail.scrollTop -= view.top + 40 - top;
    else if (bottom > view.bottom - 40) rail.scrollTop += bottom - view.bottom + 40;
}

// Track from the lowest bar: the strip, or the top bar on phones, where the hidden strip's box is all zeros.
const barBottom = (selector: string) => document.querySelector(selector)?.getBoundingClientRect().bottom ?? 0;
let frame = 0;
let shown: string | undefined;
function track() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
        const top = Math.max(barBottom('.doc-strip'), barBottom('.doc-top')) + 24;
        const atEnd = innerHeight + scrollY >= root.scrollHeight - 2;
        let found = atEnd ? headings.at(-1) : undefined;
        if (!found) for (const h of headings) if (h.getBoundingClientRect().top <= top) found = h;
        found ??= headings[0];
        if (!found || found.id === shown) return;
        shown = found.id;
        const h2 = found.tagName === 'H2' ? found : sections.findLast(h => h.compareDocumentPosition(found) & Node.DOCUMENT_POSITION_FOLLOWING);
        for (const a of document.querySelectorAll('.doc-rail a[aria-current], .doc-sheet-section[aria-current]')) a.removeAttribute('aria-current');
        for (const a of document.querySelectorAll('.doc-rail a.within')) a.classList.remove('within');
        for (const a of linksTo(found.id)) a.setAttribute('aria-current', 'location');
        if (h2) {
            for (const a of linksTo(h2.id)) {
                if (a.matches('.doc-rail a')) a.classList.add('within');
                else a.setAttribute('aria-current', 'location');
            }
            // Only the current section shows its entries, unless the reader is filtering.
            if (!filtering)
                for (const list of index?.querySelectorAll<HTMLElement>(':scope > ol > li > ol') ?? [])
                    list.hidden = list.parentElement!.firstElementChild!.getAttribute('href') !== `#${h2.id}`;
            if (pill) {
                pill.querySelector('.num')!.textContent = String(sections.indexOf(h2) + 1).padStart(2, '0');
                pill.querySelector('.text')!.textContent = label(h2);
            }
        }
        revealActive();
    });
}

// The filter searches every section's entries, in the rail and in the sections sheet.
function filterRail(q: string) {
    if (!index) return;
    filtering = !!q;
    let any = false;
    for (const li of index.querySelectorAll<HTMLElement>(':scope > ol > li')) {
        const children = [...li.querySelectorAll<HTMLElement>('ol > li')];
        for (const c of children) c.hidden = !!q && !c.textContent.toLowerCase().includes(q);
        const list = li.querySelector<HTMLElement>('ol');
        const matches = children.some(c => !c.hidden);
        if (list) list.hidden = q ? !matches : true;
        li.hidden = !!q && !matches && !li.firstElementChild!.textContent.toLowerCase().includes(q);
        any ||= !li.hidden;
    }
    index.querySelector<HTMLElement>('p.dim')!.hidden = any;
    if (!q) {
        shown = undefined;
        track();
    }
}

function filterSheet(sheet: HTMLElement, q: string) {
    let section: HTMLElement | undefined;
    let sectionMatches = false;
    const entries = [...sheet.querySelectorAll<HTMLElement>('.doc-sheet-section')];
    // A section stays when it or one of its entries matches.
    const keep = new Set<HTMLElement>();
    for (const a of entries) {
        if (!a.classList.contains('child')) {
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

for (const input of document.querySelectorAll<HTMLInputElement>('.doc-filter')) {
    input.addEventListener('input', () => {
        const q = input.value.trim().toLowerCase();
        const sheet = input.closest<HTMLElement>('.doc-sheet');
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
        pill?.classList.toggle('away', scrollY > lastY && scrollY > 200);
        lastY = scrollY;
    },
    { passive: true },
);
addEventListener('resize', track);
addEventListener('hashchange', track);
track();
document.querySelector('.doc-strip [aria-current="page"]')?.scrollIntoView({ inline: 'center', block: 'nearest' });

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

// The sheets are native modal dialogs; their buttons say whether they're open.
for (const dialog of document.querySelectorAll<HTMLDialogElement>('dialog.doc-sheet')) {
    dialog.addEventListener('close', () => {
        root.classList.remove('doc-locked');
        document.querySelector(`[data-sheet="${dialog.id}"]`)?.setAttribute('aria-expanded', 'false');
    });
}
function openSheet(button: HTMLElement) {
    const dialog = document.getElementById(button.dataset.sheet!) as HTMLDialogElement | null;
    if (!dialog) return;
    dialog.showModal();
    // The sheet itself takes focus, as a page would: focusing its filter would open the phone's keyboard.
    dialog.focus();
    root.classList.add('doc-locked');
    button.setAttribute('aria-expanded', 'true');
}

// Copying a code block copies what it shows, without removed diff lines, and shell commands without their prompts.
async function copy(button: HTMLElement) {
    const block = button.parentElement;
    const pre = block?.querySelector('pre');
    if (!block || !pre) return;
    const clone = pre.cloneNode(true) as HTMLElement;
    clone.querySelectorAll('.diff.remove, .vp-copy-ignore').forEach(el => el.remove());
    let text = clone.textContent ?? '';
    if (/language-(?:shellscript|shell|bash|sh|zsh)\b/.test(block.className)) text = text.replace(/^ *[$>] /gm, '').trim();
    try {
        await navigator.clipboard.writeText(text);
    } catch {
        // Without the Clipboard API (an insecure origin, an older browser), copy through a selected text area.
        const area = Object.assign(document.createElement('textarea'), { value: text, readOnly: true });
        area.style.cssText = 'position:fixed;opacity:0';
        document.body.append(area);
        area.select();
        document.execCommand('copy');
        area.remove();
    }
    button.classList.add('copied');
    setTimeout(() => button.classList.remove('copied'), 2000);
}

let search: Promise<typeof import('./search.ts')> | undefined;
const openSearch = () => void (search ??= import('./search.ts')).then(m => m.open());

document.addEventListener('click', e => {
    const target = e.target as HTMLElement;
    if (target.closest('[data-theme-toggle]')) return toggleTheme();
    const sheetButton = target.closest<HTMLElement>('[data-sheet]');
    if (sheetButton) return openSheet(sheetButton);
    // A tap on the backdrop lands on the dialog itself.
    if (target.matches('dialog.doc-sheet')) return (target as HTMLDialogElement).close();
    const section = target.closest<HTMLAnchorElement>('.doc-sheet-section');
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
    const copyButton = target.closest<HTMLElement>('div[class*="language-"] > button.copy');
    if (copyButton) return void copy(copyButton);
    if (target.closest('[data-search]')) openSearch();
});

// Code groups (and example tabs, which reuse their markup): the checked tab's block shows.
document.addEventListener('change', e => {
    const input = e.target as HTMLInputElement;
    const group = input.closest('.vp-code-group');
    if (!group || !input.matches('.tabs > input')) return;
    const i = [...group.querySelectorAll(':scope > .tabs > input')].indexOf(input);
    [...group.querySelector(':scope > .blocks')!.children].forEach((block, n) => block.classList.toggle('active', n === i));
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
