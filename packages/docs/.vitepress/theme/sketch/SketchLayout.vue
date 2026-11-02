<script setup lang="ts">
// Layout sketch: a docs shell that owns its whole layout, around VitePress's rendered content.
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Content, onContentUpdated, useData, useRoute, withBase } from 'vitepress';
import { VPNavBarSearch } from 'vitepress/theme';
import FonteWordmark from '../FonteWordmark.vue';
import { data as optionTools } from './options.data';
import { optionKey } from './optionKey';

const { theme, page, isDark } = useData();
const route = useRoute();

type Item = { text: string; link: string };
type Group = { text: string; items: Item[] };

const npm = (name: string) => `https://www.npmjs.com/package/${name}`;
const crate = 'https://crates.io/crates/webfont-generator';
const tools = [
    { key: 'vite', plate: 'Plugin', name: 'Vite', package: 'vite-svg-2-webfont', registry: npm('vite-svg-2-webfont') },
    { key: 'node', plate: 'npm', name: 'Node.js', package: '@atlowchemi/webfont-generator', registry: npm('@atlowchemi/webfont-generator') },
    { key: 'browser', plate: 'Wasm', name: 'Browser', package: '@atlowchemi/webfont-browser-generator', registry: npm('@atlowchemi/webfont-browser-generator') },
    { key: 'cli', plate: 'Binary', name: 'CLI', package: 'webfont-generator --features cli', registry: crate },
    { key: 'rust', plate: 'Crate', name: 'Rust', package: 'webfont-generator', registry: crate },
] as const;
const github = computed(() => (theme.value.socialLinks as { icon: string; link: string }[]).find(l => l.icon === 'github')?.link);

const normalize = (path: string) => path.replace(/(\.html|\/index)$/, '').replace(/\/$/, '') || '/';
const current = computed(() => normalize(route.path));
const section = computed(() => current.value.split('/')[1] ?? '');
const tool = computed(() => tools.find(t => t.key === section.value));

const sidebar = computed(() => (theme.value.sidebar as Record<string, Group[]>)[`/${section.value}/`] ?? []);
// The strip lists the current group's pages: a tool's own pages, or the guide group the page belongs to.
const group = computed<Group | undefined>(() => {
    const groups = tool.value ? sidebar.value.slice(0, 1) : sidebar.value.slice(0, 2);
    return groups.find(g => g.items.some(i => normalize(i.link) === current.value)) ?? groups[0];
});
const index = computed(() => group.value?.items.findIndex(i => normalize(i.link) === current.value) ?? -1);
const no = (i: number) => String(i + 1).padStart(2, '0');
const prev = computed(() => (index.value > 0 ? group.value!.items[index.value - 1] : undefined));
const next = computed(() => (group.value && index.value < group.value.items.length - 1 ? group.value.items[index.value + 1] : undefined));
const isGuide = computed(() => section.value === 'guide');
const guideGroups = computed(() => ((theme.value.sidebar as Record<string, Group[]>)['/guide/'] ?? []).slice(0, 2));
// Only the guide is read in order, so only its pages carry a number; tool pages and features are looked up.
const numbered = computed(() => isGuide.value && group.value?.text === guideGroups.value[0]?.text);

/** Switching tools keeps your place: Node's Configuration → Vite's Configuration, when it has one. */
function toolLink(key: string): string {
    const pageType = current.value.split('/')[2];
    const pages = (theme.value.sidebar as Record<string, Group[]>)[`/${key}/`]?.[0]?.items ?? [];
    const same = pageType && pages.find(i => normalize(i.link) === `/${key}/${pageType}`);
    return withBase(same ? same.link : `/${key}/`);
}

// The rail's index: the page's h2 sections, each with its h3 entries (options, functions), followed while reading.
type Entry = { id: string; text: string; children: { id: string; text: string }[] };
const sections = ref<Entry[]>([]);
const active = ref('');
const activeSection = ref('');
const filter = ref('');
const entryCount = computed(() => sections.value.reduce((n, s) => n + s.children.length, 0));
const shown = computed(() => {
    const q = filter.value.trim().toLowerCase();
    if (!q) return sections.value.map(s => ({ ...s, children: s.id === activeSection.value ? s.children : [] }));
    return sections.value
        .map(s => ({ ...s, children: s.children.filter(c => c.text.toLowerCase().includes(q)) }))
        .filter(s => s.children.length || s.text.toLowerCase().includes(q));
});
let headings: HTMLElement[] = [];
const label = (h: HTMLElement) => {
    const clone = h.cloneNode(true) as HTMLElement;
    clone.querySelector('.header-anchor')?.remove();
    return clone.textContent?.trim() ?? '';
};

function readSections() {
    headings = [...document.querySelectorAll<HTMLElement>('.sk-main .vp-doc :is(h2, h3)[id]')];
    const entries: Entry[] = [];
    for (const h of headings) {
        if (h.tagName === 'H2') entries.push({ id: h.id, text: label(h), children: [] });
        else entries.at(-1)?.children.push({ id: h.id, text: label(h) });
    }
    sections.value = entries;
    filter.value = '';
    track();
}

// The current entry is the last heading above the bars, so it stays right after any jump: links, search, "Also in".
const offset = () => (document.querySelector('.sk-strip')?.getBoundingClientRect().bottom ?? 0) + 24;
let frame = 0;
function track() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
        const top = offset();
        const atEnd = innerHeight + scrollY >= document.documentElement.scrollHeight - 2;
        let found = atEnd ? headings.at(-1) : undefined;
        if (!found) for (const h of headings) if (h.getBoundingClientRect().top <= top) found = h;
        found ??= headings[0];
        active.value = found?.id ?? '';
        activeSection.value = found?.tagName === 'H2' ? found.id : (sections.value.find(s => s.children.some(c => c.id === found?.id))?.id ?? '');
        void nextTick(revealActive);
    });
}

// Keeps the current entry in view inside the rail without moving the page.
const rail = ref<HTMLElement>();
function revealActive() {
    const box = rail.value;
    const link = box?.querySelector<HTMLElement>('a[aria-current="location"]');
    if (!box || !link || box.offsetParent === null) return;
    const { top, bottom } = link.getBoundingClientRect();
    const view = box.getBoundingClientRect();
    if (top < view.top + 40) box.scrollTop -= view.top + 40 - top;
    else if (bottom > view.bottom - 40) box.scrollTop += bottom - view.bottom + 40;
}

// Options become specimens: the description on the left, a spec card (type, default, flag, other tools) on the right.
// The card takes the facts from an option's "Type: …" list (the first list that opens with one); notes in the same list stay with the description.
const SPEC = /^(Type|Default|Required|Flags?|Manifest|Valid values|Values|Availability|Since|Env|Reference)\b(?::\s*|$)/;
function specimens() {
    const doc = document.querySelector('.sk-main .vp-doc');
    if (!doc) return;
    for (const h3 of doc.querySelectorAll<HTMLElement>('h3')) {
        if (h3.closest('.sk-option') || !h3.matches(':has(> code:first-child + .header-anchor:last-child)')) continue;
        const article = document.createElement('article');
        article.className = 'sk-option';
        article.setAttribute('aria-labelledby', h3.id);
        const desc = document.createElement('div');
        desc.className = 'sk-desc';
        h3.before(article);
        article.append(h3, desc);
        let spec: HTMLUListElement | undefined;
        while (article.nextElementSibling && !article.nextElementSibling.matches('h2, h3, .sk-option')) {
            const el = article.nextElementSibling as HTMLElement;
            if (!spec && el.matches('ul') && SPEC.test(el.firstElementChild?.textContent ?? '')) spec = el as HTMLUListElement;
            desc.append(el);
        }
        const dl = document.createElement('dl');
        dl.className = 'sk-spec';
        for (const li of spec ? [...spec.children] : []) {
            const text = (li.textContent ?? '').trim();
            const key = text.match(SPEC)?.[1];
            if (!key) continue;
            const row = document.createElement('div');
            // A bare "Required" carries no value of its own.
            const html = text === key ? 'yes' : li.innerHTML.replace(new RegExp(`^\\s*${key}\\b:\\s*`), '');
            row.innerHTML = `<dt>${key}</dt><dd>${html}</dd>`;
            dl.append(row);
            li.remove();
        }
        if (spec && !spec.children.length) spec.remove();
        const name = h3.querySelector('code')?.textContent ?? '';
        const others = (optionTools[optionKey(name)] ?? []).filter(o => o.tool !== section.value);
        if (others.length) {
            const row = document.createElement('div');
            row.className = 'also';
            row.innerHTML = `<dt>Also in</dt><dd>${others
                .map(o => {
                    const toolName = tools.find(t => t.key === o.tool)?.name ?? o.tool;
                    const alias = o.name.toLowerCase().replace(/[^a-z0-9]/g, '') === name.toLowerCase().replace(/[^a-z0-9]/g, '') ? '' : ` <code>${o.name}</code>`;
                    return `<a href="${withBase(`/${o.tool}/configuration#${o.id}`)}">${toolName}${alias}</a>`;
                })
                .join('')}</dd>`;
            dl.append(row);
        }
        if (dl.children.length) article.append(dl);
        else article.classList.add('bare');
    }
}

onContentUpdated(() => {
    specimens();
    // The specimens moved the page under the router's jump to a #hash; land on the heading again.
    if (location.hash) document.getElementById(decodeURIComponent(location.hash.slice(1)))?.scrollIntoView();
    readSections();
    void nextTick(() => document.querySelector('.sk-strip [aria-current="page"]')?.scrollIntoView({ inline: 'center', block: 'nearest' }));
});

// Phones: the section pill steps aside while you scroll down, and comes back when you scroll up.
const pillHidden = ref(false);
let lastY = 0;
function onScroll() {
    track();
    pillHidden.value = scrollY > lastY && scrollY > 200;
    lastY = scrollY;
}

// Mobile sheets: tools, and the page's sections. Each is a modal dialog: focus moves in, stays in, and goes back.
const sheet = ref<'' | 'tools' | 'sections'>('');
const sheetEl = ref<HTMLElement>();
let opener: HTMLElement | null = null;
watch(
    () => route.path,
    () => (sheet.value = ''),
);
watch(sheet, async (open, was) => {
    document.documentElement.classList.toggle('sk-locked', !!open);
    if (open && !was) {
        opener = document.activeElement as HTMLElement | null;
        await nextTick();
        sheetEl.value?.focus();
    } else if (!open && was) {
        // The bars stop being inert on the next render; focus can go back to them after that.
        const back = opener;
        opener = null;
        await nextTick();
        back?.focus();
    }
});
const focusables = () => [...(sheetEl.value?.querySelectorAll<HTMLElement>('a[href], button, input') ?? [])];
function onKeydown(e: KeyboardEvent) {
    if (!sheet.value) return;
    if (e.key === 'Escape') sheet.value = '';
    if (e.key !== 'Tab') return;
    const items = focusables();
    const first = items[0];
    const last = items.at(-1);
    if (e.shiftKey && (document.activeElement === first || document.activeElement === sheetEl.value)) {
        e.preventDefault();
        last?.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first?.focus();
    }
}

// A deep link lands before fonts and late blocks settle, which pushes the heading down; land again once they have,
// unless the reader has already moved.
let moved = false;
const markMoved = () => (moved = true);
async function settleHash() {
    if (!location.hash) return;
    for (const type of ['wheel', 'touchstart', 'keydown']) addEventListener(type, markMoved, { once: true, passive: true });
    await document.fonts.ready;
    await new Promise(resolve => setTimeout(resolve, 300));
    if (!moved) document.getElementById(decodeURIComponent(location.hash.slice(1)))?.scrollIntoView();
}

onMounted(() => {
    void settleHash();
    addEventListener('keydown', onKeydown);
    addEventListener('scroll', onScroll, { passive: true });
    addEventListener('resize', track);
    addEventListener('hashchange', track);
});
onBeforeUnmount(() => {
    removeEventListener('keydown', onKeydown);
    removeEventListener('scroll', onScroll);
    removeEventListener('resize', track);
    removeEventListener('hashchange', track);
    cancelAnimationFrame(frame);
    document.documentElement.classList.remove('sk-locked');
});

const activeEntry = computed(() => sections.value.findIndex(s => s.id === activeSection.value));
function go(id: string) {
    // Focus moves to the section, not back to the pill.
    opener = null;
    sheet.value = '';
    const target = document.getElementById(id);
    if (!target) return;
    const smooth = !matchMedia('(prefers-reduced-motion: reduce)').matches;
    // Wait for the sheet to give focus back before moving it to the section.
    void nextTick(() => {
        target.scrollIntoView({ behavior: smooth ? 'smooth' : 'auto' });
        target.setAttribute('tabindex', '-1');
        target.focus({ preventScroll: true });
    });
    history.replaceState(null, '', `#${id}`);
}

const toggleTheme = () => (isDark.value = !isDark.value);
const themeLabel = computed(() => (isDark.value ? 'Switch to light theme' : 'Switch to dark theme'));
</script>

<template>
    <div class="sk">
        <a class="sk-skip" href="#sk-content">Skip to content</a>
        <header class="sk-top" :inert="sheet ? true : undefined">
            <a class="sk-wordmark" :href="withBase('/')" aria-label="fonte home"><FonteWordmark /></a>
            <button class="sk-tool-button" type="button" aria-haspopup="dialog" :aria-expanded="sheet === 'tools'" @click="sheet = sheet === 'tools' ? '' : 'tools'">
                <span class="plate">{{ tool?.plate ?? 'Start here' }}</span
                ><strong>{{ tool?.name ?? 'Guide' }}</strong
                ><svg class="caret" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="m6 9 6 6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
            </button>
            <nav class="sk-drawers" aria-label="Tools">
                <a class="sk-drawer" :href="withBase('/guide/')" :aria-current="isGuide ? 'true' : undefined"><span class="plate">Start here</span><strong>Guide</strong></a>
                <a v-for="t in tools" :key="t.key" class="sk-drawer" :href="toolLink(t.key)" :aria-current="t.key === section ? 'true' : undefined"
                    ><span class="plate">{{ t.plate }}</span
                    ><strong>{{ t.name }}</strong></a
                >
            </nav>
            <a class="sk-play" :href="withBase('/playground')" aria-label="Playground: generate a font in your browser">
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5.5v13l10.5-6.5Z" fill="currentColor" /></svg>
                <span class="label" aria-hidden="true"><span class="plate">Try it</span><strong>Playground</strong></span>
            </a>
            <div class="sk-search"><VPNavBarSearch /></div>
            <a v-if="github" class="sk-icon sk-github" :href="github" target="_blank" rel="noreferrer" aria-label="fonte on GitHub">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path
                        fill="currentColor"
                        d="M12 .3a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2c-3.3.7-4-1.6-4-1.6-.6-1.4-1.4-1.8-1.4-1.8-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.8 1.3 3.5 1 0-.8.4-1.3.7-1.6-2.7-.3-5.5-1.3-5.5-6 0-1.2.5-2.3 1.3-3.1-.2-.4-.6-1.6 0-3.2 0 0 1-.3 3.4 1.2a11.5 11.5 0 0 1 6 0C17.3 4.6 18.3 5 18.3 5c.6 1.6.2 2.8.1 3.2.8.8 1.3 1.9 1.3 3.1 0 4.6-2.8 5.6-5.5 5.9.5.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .3"
                    />
                </svg>
            </a>
            <button class="sk-icon sk-theme" type="button" :aria-label="themeLabel" :title="themeLabel" @click="toggleTheme">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                    <circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="1.6" />
                    <path d="M12 4a8 8 0 0 1 0 16Z" fill="currentColor" />
                </svg>
            </button>
        </header>

        <nav class="sk-strip" :aria-label="`${group?.text ?? ''} pages`" :inert="sheet ? true : undefined">
            <template v-if="isGuide">
                <!-- The guide has two shelves; the switch says which one you're on, the tabs are that shelf's pages. -->
                <span class="sk-shelves" role="group" aria-label="Guide shelves">
                    <a v-for="g in guideGroups" :key="g.text" :href="withBase(g.items[0]!.link)" :aria-current="g.text === group?.text ? 'true' : undefined">{{ g.text }}</a>
                </span>
            </template>
            <a v-for="(item, i) in group?.items ?? []" :key="item.link" :href="withBase(item.link)" :aria-current="normalize(item.link) === current ? 'page' : undefined"
                ><span v-if="numbered" class="n">{{ no(i) }}</span
                >{{ item.text }}</a
            >
            <a v-if="!isGuide" class="guide" :href="withBase('/guide/')">Guide &amp; features →</a>
        </nav>

        <div class="sk-page" :inert="sheet ? true : undefined">
            <aside class="sk-rail" ref="rail">
                <template v-if="numbered">
                    <div class="mono dim">No.</div>
                    <div class="num">{{ no(index) }}</div>
                </template>
                <div v-else class="mono dim">{{ tool ? tool.plate : group?.text }}</div>
                <div class="name">{{ group?.items[index]?.text ?? page.title }}</div>
                <a v-if="tool" class="mono dim tool" :href="tool.registry" target="_blank" rel="noreferrer">{{ tool.name }} · {{ tool.package }} ↗</a>
                <div v-else-if="numbered" class="mono dim tool">{{ group?.text }}</div>
                <nav v-if="sections.length" class="index" aria-label="On this page">
                    <input v-if="entryCount >= 10" v-model="filter" class="sk-filter" type="search" placeholder="Filter this page" aria-label="Filter this page's sections" />
                    <ol>
                        <li v-for="(s, i) in shown" :key="s.id">
                            <a :href="`#${s.id}`" :class="{ within: s.id === activeSection }" :aria-current="s.id === active ? 'location' : undefined"
                                ><span class="n">{{ no(sections.findIndex(x => x.id === s.id)) }}</span
                                >{{ s.text }}</a
                            >
                            <ol v-if="s.children.length">
                                <li v-for="c in s.children" :key="c.id">
                                    <a :href="`#${c.id}`" :aria-current="c.id === active ? 'location' : undefined">{{ c.text }}</a>
                                </li>
                            </ol>
                        </li>
                    </ol>
                    <p v-if="!shown.length" class="dim">Nothing matches “{{ filter }}”.</p>
                </nav>
            </aside>

            <main id="sk-content" class="sk-main" tabindex="-1">
                <div class="vp-doc"><Content /></div>
                <nav class="sk-next" aria-label="Previous and next pages">
                    <a v-if="prev" :href="withBase(prev.link)"
                        ><span class="mono">← {{ numbered ? `No. ${no(index - 1)}` : 'Previous' }}</span
                        ><strong>{{ prev.text }}</strong></a
                    >
                    <span v-else />
                    <a v-if="next" class="right" :href="withBase(next.link)"
                        ><span class="mono">{{ numbered ? `No. ${no(index + 1)}` : 'Next' }} →</span><strong>{{ next.text }}</strong></a
                    >
                </nav>
            </main>
        </div>

        <!-- Phones: the rail's index becomes a pill that names the current section and opens a sheet. -->
        <button
            v-if="sections.length"
            class="sk-pill"
            :class="{ away: pillHidden }"
            type="button"
            aria-haspopup="dialog"
            :aria-expanded="sheet === 'sections'"
            :inert="sheet ? true : undefined"
            @click="sheet = 'sections'"
        >
            <span class="num">{{ no(Math.max(0, activeEntry)) }}</span
            >{{ sections[activeEntry]?.text ?? 'On this page' }}<span aria-hidden="true">▴</span>
        </button>

        <Transition name="sk-fade">
            <div v-if="sheet" class="sk-scrim" @click="sheet = ''" />
        </Transition>
        <Transition name="sk-sheet">
            <div v-if="sheet === 'tools'" ref="sheetEl" class="sk-sheet" role="dialog" aria-modal="true" aria-labelledby="sk-sheet-title" tabindex="-1">
                <div id="sk-sheet-title" class="sk-sheet-title mono">Tools</div>
                <a class="sk-sheet-tool" :href="withBase('/guide/')" :aria-current="isGuide ? 'true' : undefined"
                    ><span class="plate">Start here</span><strong>Guide</strong><span class="pkg mono">Concepts and features</span></a
                >
                <a v-for="t in tools" :key="t.key" class="sk-sheet-tool" :href="toolLink(t.key)" :aria-current="t.key === section ? 'true' : undefined"
                    ><span class="plate">{{ t.plate }}</span
                    ><strong>{{ t.name }}</strong
                    ><span class="pkg mono">{{ t.package }}</span></a
                >
                <a class="sk-sheet-tool play" :href="withBase('/playground')"
                    ><span class="plate">Try it</span><strong>Playground</strong><span class="pkg mono">Generate a font in your browser</span></a
                >
                <div class="sk-sheet-links">
                    <a v-if="github" :href="github" target="_blank" rel="noreferrer"><span class="plate">Source</span>GitHub ↗</a>
                    <a v-if="tool" :href="tool.registry" target="_blank" rel="noreferrer"
                        ><span class="plate">Package</span>{{ tool.registry.includes('crates.io') ? 'crates.io' : 'npm' }} ↗</a
                    >
                    <button type="button" @click="toggleTheme"><span class="plate">Theme</span>{{ isDark ? 'Use light' : 'Use dark' }}</button>
                </div>
            </div>
            <div v-else-if="sheet === 'sections'" ref="sheetEl" class="sk-sheet" role="dialog" aria-modal="true" aria-labelledby="sk-sheet-title" tabindex="-1">
                <div id="sk-sheet-title" class="sk-sheet-title mono">On this page · {{ group?.items[index]?.text ?? page.title }}</div>
                <input v-if="entryCount >= 10" v-model="filter" class="sk-filter" type="search" placeholder="Filter this page" aria-label="Filter this page's sections" />
                <template v-for="s in filter ? shown : sections" :key="s.id">
                    <a class="sk-sheet-section" :aria-current="s.id === activeSection ? 'location' : undefined" :href="`#${s.id}`" @click.prevent="go(s.id)"
                        ><span class="n">{{ no(sections.findIndex(x => x.id === s.id)) }}</span
                        >{{ s.text }}</a
                    >
                    <a
                        v-for="c in s.children"
                        :key="c.id"
                        class="sk-sheet-section child"
                        :aria-current="c.id === active ? 'location' : undefined"
                        :href="`#${c.id}`"
                        @click.prevent="go(c.id)"
                        >{{ c.text }}</a
                    >
                </template>
            </div>
        </Transition>
    </div>
</template>

<style src="./sketch.css"></style>
