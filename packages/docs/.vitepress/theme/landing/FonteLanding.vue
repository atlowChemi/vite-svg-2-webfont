<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useData, withBase } from 'vitepress';
import FonteWordmark from '../FonteWordmark.vue';
import { arrowDown, packageMarks, trayIcons } from './icons';

const repo = 'https://github.com/atlowChemi/vite-svg-2-webfont';

const features = [
    {
        icon: '/native-workflow.svg',
        title: 'Native speed',
        details: 'A ground-up Rust rewrite of the webfont pipeline, with parallel font conversion and native path normalization.',
    },
    {
        icon: '/configurable.svg',
        title: 'Every detail configurable',
        details: 'File globs, destinations, selectors, class prefixes, codepoints, font formats, and your own templates.',
    },
    {
        icon: '/in-browser.svg',
        title: 'Right in the browser',
        details: 'The same engine compiled to WebAssembly casts fonts in a web worker, with no server and no build step.',
    },
];
const icons = Object.entries(trayIcons);

const drawers = [
    {
        key: 'cli',
        mark: packageMarks.rust,
        plate: 'Rust',
        title: 'CLI',
        use: 'CI jobs or one-off local runs, from any language',
        link: '/webfont-generator/cli',
    },
    {
        key: 'node',
        mark: packageMarks.node,
        plate: 'Node',
        title: 'Build scripts',
        use: 'Your own tooling, at native speed',
        link: '/webfont-generator/node',
    },
    {
        key: 'wasm',
        mark: packageMarks.wasm,
        plate: 'WASM',
        title: 'Browser',
        use: 'Casts from in-memory files, no file system needed',
        link: '/webfont-generator/browser',
    },
    {
        key: 'vite',
        mark: packageMarks.vite,
        plate: 'Vite',
        title: 'Plugin',
        use: 'Runs inside the dev server, icon edits hot-reload on save',
        link: '/getting-started',
    },
];
const activeKey = ref('cli');
const activeDrawer = computed(() => drawers.find(drawer => drawer.key === activeKey.value) ?? drawers[0]!);

// The T's metal level in its 100×86 box: empty at the stem's foot, full at the crossbar.
const EMPTY = 90;
const FULL = 22;

function wave(level: number, amplitude: number, phase: number): string {
    const points: string[] = [];
    for (let x = 0; x <= 100; x += 4) {
        const y = level + amplitude * Math.sin((x / 25) * Math.PI + phase) + amplitude * 0.5 * Math.sin((x / 16.7) * Math.PI - phase * 1.3);
        points.push(`${x ? 'L' : 'M'}${x} ${y.toFixed(2)}`);
    }
    return points.join(' ');
}

const emptySurface = wave(EMPTY, 0, 0);
const surface = ref(emptySurface);
const degrees = ref('340°C');
const ready = ref(false);

const root = useTemplateRef<HTMLElement>('root');
const hero = useTemplateRef<HTMLElement>('hero');
const stage = useTemplateRef<HTMLElement>('stage');
const placeholder = useTemplateRef<HTMLElement>('placeholder');
const flyer = useTemplateRef<InstanceType<typeof FonteWordmark>>('flyer');
const trayItems = useTemplateRef<HTMLElement[]>('trayItems');
const hint = useTemplateRef<HTMLElement>('hint');
const result = useTemplateRef<HTMLElement>('result');

const { isDark } = useData();

type Rgb = [number, number, number];
// Accent temperatures, hot to cold. Light mode uses darker stops so the accent stays readable on ash.
const heat: Record<'dark' | 'light', { hot: Rgb; ember: Rgb; cold: Rgb; ink: Rgb }> = {
    dark: { hot: [255, 106, 19], ember: [255, 176, 46], cold: [138, 130, 144], ink: [243, 236, 226] },
    light: { hot: [194, 65, 12], ember: [161, 92, 0], cold: [125, 117, 130], ink: [28, 26, 31] },
};
const clamp = (value: number) => Math.min(1, Math.max(0, value));
const mix = (a: Rgb, b: Rgb, t: number) => `rgb(${a.map((v, i) => Math.round(v + (b[i]! - v) * t)).join(' ')})`;

type Dock = { y0: number; y1: number; fx: number; fy: number; tx: number; ty: number; k: number };
let dock: Dock | undefined;
let reduce = false;
let scrollDriven = false;
let level = EMPTY;
let target = EMPTY;
let agitation = 0;
let phase = 0;
let landed = 0;
let raf = 0;

// Where the hero wordmark sits while the stage is pinned, where the nav keeps its slot, and the scroll span between.
function measure() {
    const navMark = document.querySelector<SVGElement>('.VPNavBarTitle .fonte-wordmark');
    const flyerEl = flyer.value?.$el as SVGElement | undefined;
    if (!navMark || !flyerEl || !hero.value || !stage.value || !placeholder.value) return;
    const stickTop = parseFloat(getComputedStyle(stage.value).top);
    const heroTop = hero.value.getBoundingClientRect().top + scrollY;
    const y0 = heroTop - stickTop;
    const y1 = heroTop + hero.value.offsetHeight - stickTop - stage.value.offsetHeight;
    const nav = navMark.getBoundingClientRect();
    dock = {
        y0,
        y1,
        fx: stage.value.getBoundingClientRect().left + placeholder.value.offsetLeft,
        fy: stickTop + placeholder.value.offsetTop,
        tx: nav.left,
        ty: nav.top,
        k: nav.height / placeholder.value.offsetHeight,
    };
    const vars = { '--from-x': dock.fx, '--from-y': dock.fy, '--to-x': dock.tx, '--to-y': dock.ty, '--dock-a': y0 + (y1 - y0) * 0.84, '--dock-b': y1 };
    for (const [name, value] of Object.entries(vars)) flyerEl.style.setProperty(name, `${value}px`);
    flyerEl.style.setProperty('--to-k', String(dock.k));
    ready.value = true;
}

function update() {
    const flyerEl = flyer.value?.$el as SVGElement | undefined;
    if (!dock || !flyerEl || !stage.value) return;
    const progress = reduce ? 1 : clamp((scrollY - dock.y0) / (dock.y1 - dock.y0));
    const colors = heat[isDark.value ? 'dark' : 'light'];

    // Each icon falls into the mouth of the T, heating up on the way, and raises the metal when it lands.
    const stageBox = stage.value.getBoundingClientRect();
    const mold = flyerEl.querySelector('.mold')!.getBoundingClientRect();
    const mouthX = mold.left + mold.width / 2 - stageBox.left;
    const mouthY = mold.top + mold.height * 0.22 - stageBox.top;
    let filled = 0;
    let count = 0;
    for (const [i, item] of (trayItems.value ?? []).entries()) {
        const q = clamp((progress - (0.06 + i * 0.08)) / 0.14);
        const dx = (mouthX - item.offsetLeft - item.offsetWidth / 2) * (1 - (1 - q) ** 2);
        const dy = (mouthY - item.offsetTop - item.offsetHeight / 2) * q * q;
        item.style.transform = `translate(${dx}px, ${dy}px) scale(${1 - 0.7 * q})`;
        item.style.color = q < 0.5 ? mix(colors.ink, colors.ember, q * 2) : mix(colors.ember, heat.dark.hot, (q - 0.5) * 2);
        item.style.opacity = String(q > 0.85 ? (1 - q) / 0.15 : 1);
        filled += clamp((q - 0.85) / 0.15);
        if (q >= 1) count++;
    }
    if (count > landed) agitation = 2.4;
    landed = count;
    target = EMPTY - ((EMPTY - FULL) / icons.length) * filled;

    if (hint.value) hint.value.style.opacity = String(clamp(1 - progress / 0.04));
    if (result.value) {
        const shown = clamp((progress - 0.62) / 0.14);
        result.value.style.opacity = String(shown);
        result.value.style.transform = `translateY(${(1 - shown) * 16}px)`;
        result.value.inert = shown < 0.5;
    }

    if (!scrollDriven) {
        let d = clamp((scrollY - (dock.y0 + (dock.y1 - dock.y0) * 0.84)) / ((dock.y1 - dock.y0) * 0.16));
        if (reduce) d = d >= 1 ? 1 : 0;
        const lerp = (a: number, b: number) => a + (b - a) * d;
        flyerEl.style.transform = `translate(${lerp(dock.fx, dock.tx)}px, ${lerp(dock.fy, dock.ty)}px) scale(${lerp(1, dock.k)})`;
        flyerEl.style.setProperty('--fonte-mold-width', String(lerp(1.4, 2.4)));
    }

    // The page cools once the cast is done: molten through ember to cold metal at the footer.
    const maxScroll = document.documentElement.scrollHeight - innerHeight;
    const t = scrollY <= dock.y1 ? 1 : clamp(1 - (scrollY - dock.y1) / Math.max(1, maxScroll - dock.y1));
    root.value?.style.setProperty('--acc', t > 0.5 ? mix(colors.ember, colors.hot, (t - 0.5) / 0.5) : mix(colors.cold, colors.ember, t / 0.5));
    degrees.value = `${Math.round(20 + t * 320)}°C`;
}

function frame() {
    level += (target - level) * 0.12;
    agitation *= 0.97;
    phase += 0.035;
    surface.value = wave(level, (level < EMPTY - 1 ? 1.3 : 0) + agitation, phase);
    if (!reduce) raf = requestAnimationFrame(frame);
}

function relayout() {
    measure();
    update();
}

onMounted(() => {
    reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
    scrollDriven = CSS.supports('animation-timeline: scroll()');
    document.documentElement.classList.add('fonte-landing');
    addEventListener('scroll', update, { passive: true });
    addEventListener('resize', relayout);
    void document.fonts.ready.then(relayout);
    relayout();
    if (reduce) level = target;
    frame();
});

watch(ready, value => document.documentElement.classList.toggle('fonte-landing-docked', value));
watch(isDark, update);

onBeforeUnmount(() => {
    cancelAnimationFrame(raf);
    removeEventListener('scroll', update);
    removeEventListener('resize', relayout);
    document.documentElement.classList.remove('fonte-landing', 'fonte-landing-docked');
});
</script>

<template>
    <div ref="root" class="landing">
        <FonteWordmark ref="flyer" class="flyer" :class="{ ready }" :surface aria-hidden="true" />

        <section ref="hero" class="sec hero">
            <div class="rail mono">
                <span class="no">No.</span><span class="num">01</span><span class="name">Casting</span><span class="deg">{{ degrees }}</span>
            </div>
            <div ref="stage" class="stage">
                <div class="tray" aria-hidden="true">
                    <span v-for="[name, paths] in icons" :key="name" ref="trayItems" class="tray-item">
                        <svg class="stroke-icon" viewBox="0 0 24 24"><path v-for="d in paths" :key="d" :d /></svg>
                    </span>
                </div>
                <div ref="placeholder" class="placeholder">
                    <FonteWordmark :class="{ hidden: ready }" :surface="emptySurface" />
                </div>
                <h1 class="tag">Melted SVGs, cast into fonts</h1>
                <p class="sub">A Rust engine for icon fonts, for the CLI, Node, the browser, and Vite.</p>
                <div ref="hint" class="hint mono" aria-hidden="true">
                    <span>scroll to cast</span>
                    <svg class="stroke-icon" viewBox="0 0 24 24"><path v-for="d in arrowDown" :key="d" :d /></svg>
                </div>
                <div ref="result" class="result">
                    <div class="result-head mono">
                        <span>my-icons.woff2</span><span>{{ icons.length }} glyphs</span>
                    </div>
                    <div class="cells">
                        <div v-for="([name, paths], i) in icons" :key="name" class="cell">
                            <svg class="stroke-icon" viewBox="0 0 24 24" aria-hidden="true"><path v-for="d in paths" :key="d" :d /></svg>
                            <code class="mono">U+E00{{ i + 1 }}</code>
                        </div>
                    </div>
                    <div class="actions">
                        <a class="button brand" href="/getting-started">Get started</a>
                        <a class="button" href="/which-package">Which package do I need?</a>
                    </div>
                </div>
            </div>
        </section>

        <section class="sec">
            <div class="rail mono">
                <span class="no">No.</span><span class="num">02</span><span class="name">Setting</span><span class="deg">{{ degrees }}</span>
            </div>
            <div class="body">
                <div class="heading">
                    <img class="illustration" :src="withBase('/engine-anywhere.svg')" alt="" />
                    <div>
                        <h2>What are you casting into?</h2>
                        <p class="lede">One engine, four drawers in the type case.</p>
                    </div>
                </div>
                <div class="drawers">
                    <button v-for="drawer in drawers" :key="drawer.key" class="drawer" type="button" :aria-pressed="drawer.key === activeKey" @click="activeKey = drawer.key">
                        <span class="drawer-top">
                            <svg class="mark" viewBox="0 0 24 24" aria-hidden="true"><path :d="drawer.mark" /></svg>
                            <span class="plate mono">{{ drawer.plate }}</span>
                        </span>
                        <strong>{{ drawer.title }}</strong>
                        <span class="use">{{ drawer.use }}</span>
                    </button>
                </div>
                <!-- Snippets come from index.md as slots, so Shiki highlights them like any other docs code block. -->
                <div class="snippet vp-doc" aria-live="polite"><slot :name="activeDrawer.key" /></div>
                <a class="more" :href="activeDrawer.link">Read the {{ activeDrawer.title }} guide →</a>
            </div>
        </section>

        <section class="sec">
            <div class="rail mono">
                <span class="no">No.</span><span class="num">03</span><span class="name">Foundry</span><span class="deg">{{ degrees }}</span>
            </div>
            <div class="body">
                <h2>Why cast with fonte</h2>
                <p class="lede">A foundry built for icon sets.</p>
                <div class="features">
                    <div v-for="feature in features" :key="feature.title" class="feature">
                        <img class="illustration" :src="withBase(feature.icon)" alt="" />
                        <h3>{{ feature.title }}</h3>
                        <p>{{ feature.details }}</p>
                    </div>
                </div>
            </div>
        </section>

        <section class="sec">
            <div class="rail mono">
                <span class="no">No.</span><span class="num">04</span><span class="name">Proof</span><span class="deg">{{ degrees }}</span>
            </div>
            <div class="body">
                <div class="heading">
                    <img class="illustration" :src="withBase('/flexible-output.svg')" alt="" />
                    <div>
                        <h2>What comes out of the mold</h2>
                        <p class="lede">A folder of SVGs in, a font family and everything around it out.</p>
                    </div>
                </div>
                <div class="files">
                    <div>
                        <div class="file">
                            <span class="ext mono">fonts</span>
                            <span class="what chips mono"
                                ><span v-for="format in ['woff2', 'woff', 'ttf', 'eot', 'svg']" :key="format" class="chip">{{ format }}</span></span
                            >
                        </div>
                        <div class="file"><span class="ext mono">.css .scss</span><span class="what">A class for every glyph, or your own template</span></div>
                        <div class="file"><span class="ext mono">.html</span><span class="what">A preview page of the whole set</span></div>
                        <div class="file"><span class="ext mono">.ts</span><span class="what">Icon names and a union type, for type-safe components</span></div>
                    </div>
                    <div class="snippet vp-doc"><slot name="outputs" /></div>
                </div>
            </div>
        </section>

        <section class="sec colophon">
            <div class="rail mono">
                <span class="no">No.</span><span class="num">05</span><span class="name">Colophon</span><span class="deg">{{ degrees }}</span>
            </div>
            <div class="body">
                <h2>Colophon</h2>
                <p class="lede">The notes a type specimen ends with.</p>
                <p>
                    <b>The name.</b> In French, <span class="fonte">fonte</span> is a casting: metal melted and poured into a mold. The same word means melting, as in
                    <i>la fonte des neiges</i>, the snowmelt, and cast iron itself.
                </p>
                <p>
                    <b>The type.</b> Founders cast letters this way for five centuries. A set of type in one size and style was <i>une fonte</i>, and English took its
                    <i>font</i> from it.
                </p>
                <p>
                    <b>The work.</b> <span class="fonte">fonte</span> does the same with vectors. Your SVGs are melted down to outlines and cast into glyphs, by an engine written
                    in Rust.
                </p>
            </div>
        </section>

        <footer class="footer">
            <span class="footer-name">fonte</span>
            <a href="/getting-started">Docs</a>
            <a :href="repo">GitHub</a>
            <a href="/changelog">Changelog</a>
            <span class="license mono">MIT license · © Chemi Atlow</span>
        </footer>
    </div>
</template>

<style>
/* The flyer animates the mold's stroke width, so the custom property has to interpolate. */
@property --fonte-mold-width {
    syntax: '<number>';
    inherits: true;
    initial-value: 2.4;
}

/* The landing page owns the nav wordmark: the hero's flyer docks into its slot. */
html.fonte-landing .VPNavBarTitle .fonte-wordmark {
    transition: opacity 0.25s;
}

html.fonte-landing-docked .VPNavBarTitle .fonte-wordmark {
    opacity: 0;
}

/*
 * The mobile menu covers the page but not the fixed flyer. While the menu button is pressed, cross-fade to the nav's
 * own wordmark, timed like VitePress's 0.25s menu fade. Keying on the button, not the menu element, starts the fade
 * on click instead of after the menu's leave transition.
 */
html.fonte-landing:has(.VPNavBarHamburger[aria-expanded='true']) .flyer {
    opacity: 0;
}

html.fonte-landing-docked:has(.VPNavBarHamburger[aria-expanded='true']) .VPNavBarTitle .fonte-wordmark {
    opacity: 1;
}

/* Below 960px VitePress lets the nav scroll away; keep it pinned so the wordmark has a slot to dock into. */
@media (max-width: 959px) {
    html.fonte-landing .VPNav {
        position: sticky;
        top: 0;
    }
}
</style>

<style scoped>
.landing {
    --acc: var(--fonte-molten);
    --rail-top: calc(var(--vp-nav-height) + 28px);
    --hero-wordmark-height: clamp(56px, 12vw, 140px);
    font-family: var(--fonte-font-display);
    font-size: 17px;
    line-height: 1.5;
    color: var(--vp-c-text-1);
}

.mono {
    font-family: var(--vp-font-family-mono);
    font-size: 12px;
    letter-spacing: 0.03em;
}

.stroke-icon {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
}

.flyer {
    position: fixed;
    pointer-events: none;
    transition: opacity 0.25s;
    top: 0;
    left: 0;
    z-index: 40;
    height: var(--hero-wordmark-height);
    width: auto;
    transform-origin: 0 0;
    transform: translate(var(--from-x), var(--from-y));
    --fonte-mold-width: 1.4;
    visibility: hidden;
    will-change: transform;
}

.flyer.ready {
    visibility: visible;
}

@keyframes dock {
    from {
        transform: translate(var(--from-x), var(--from-y));
        --fonte-mold-width: 1.4;
    }
    to {
        transform: translate(var(--to-x), var(--to-y)) scale(var(--to-k));
        --fonte-mold-width: 2.4;
    }
}

@supports (animation-timeline: scroll()) {
    .flyer {
        animation: dock linear both;
        animation-timeline: scroll(root);
        animation-range: var(--dock-a) var(--dock-b);
    }
}

@media (prefers-reduced-motion: reduce) {
    .flyer {
        animation-timing-function: steps(1, jump-end);
    }
}

.sec {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr);
    padding: 0 32px;
    border-top: 1px solid var(--vp-c-divider);
}

.rail {
    position: sticky;
    top: var(--rail-top);
    align-self: start;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 28px 0;
    color: var(--vp-c-text-3);
}

.rail .no {
    font-size: 13px;
}

.rail .num {
    font-family: var(--fonte-font-display);
    font-size: 40px;
    font-weight: 600;
    line-height: 1;
    color: var(--acc);
}

.rail .name {
    margin-top: 6px;
    font-size: 15px;
    color: var(--vp-c-text-2);
}

.rail .deg {
    margin-top: 10px;
    color: var(--acc);
}

.body {
    max-width: 880px;
    padding: 56px 0 72px;
}

h2 {
    margin: 0;
    font-size: 34px;
    font-weight: 600;
    line-height: 1.15;
}

.lede {
    margin: 6px 0 28px;
    color: var(--vp-c-text-2);
}

.illustration {
    width: 64px;
    height: 64px;
}

.heading {
    display: flex;
    align-items: flex-start;
    gap: 18px;
}

.heading .illustration {
    flex: none;
}

.features {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 24px;
}

.feature h3 {
    margin: 12px 0 4px;
    font-size: 19px;
    font-weight: 600;
}

.feature p {
    margin: 0;
    font-size: 15px;
    line-height: 1.45;
    color: var(--vp-c-text-2);
}

.hero {
    height: 300vh;
    border-top: 0;
    align-content: start;
}

.hero .rail {
    top: var(--vp-nav-height);
    padding-top: 56px;
}

.stage {
    position: sticky;
    top: var(--vp-nav-height);
    height: calc(100vh - var(--vp-nav-height));
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
}

.tray {
    display: flex;
    gap: 28px;
    height: 44px;
    align-items: center;
}

.tray-item {
    display: block;
    width: 32px;
    height: 32px;
    color: var(--vp-c-text-1);
    will-change: transform;
}

.tray-item svg {
    display: block;
    width: 100%;
    height: 100%;
}

.placeholder {
    margin-top: 12px;
}

.placeholder :deep(.fonte-wordmark) {
    height: var(--hero-wordmark-height);
    width: auto;
}

.placeholder .hidden {
    visibility: hidden;
}

.tag {
    margin: 10px 0 0;
    font-size: clamp(22px, 3vw, 30px);
    font-weight: 600;
    line-height: 1.2;
}

.sub {
    margin: 4px 0 0;
    color: var(--vp-c-text-2);
}

.hint {
    margin-top: 28px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    color: var(--vp-c-text-3);
}

.hint svg {
    width: 22px;
    height: 22px;
    color: var(--acc);
    animation: bob 1.6s ease-in-out infinite;
}

@keyframes bob {
    50% {
        transform: translateY(6px);
    }
}

@media (prefers-reduced-motion: reduce) {
    .hint svg {
        animation: none;
    }
}

.result {
    width: min(620px, 100%);
    margin-top: 22px;
    opacity: 0;
}

.result-head {
    display: flex;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px dashed var(--vp-c-divider);
    color: var(--vp-c-text-3);
}

.cells {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 8px;
    margin-top: 10px;
}

.cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 10px 4px 8px;
    border-radius: 4px;
    background: var(--vp-c-bg-soft);
}

.cell svg {
    width: 26px;
    height: 26px;
    color: var(--vp-c-brand-1);
}

.cell code {
    color: var(--vp-c-text-2);
}

.actions {
    display: flex;
    justify-content: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 18px;
}

.button {
    padding: 7px 16px;
    border: 1px solid var(--vp-c-border);
    border-radius: 4px;
    font-weight: 600;
    font-size: 15px;
    color: var(--vp-c-text-1);
    text-decoration: none;
}

.button:hover {
    border-color: var(--acc);
}

.button.brand {
    border-color: transparent;
    background: var(--vp-button-brand-bg);
    color: var(--vp-button-brand-text);
}

.button.brand:hover {
    background: var(--vp-button-brand-hover-bg);
}

.drawers {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
}

.drawer {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    border: 1px solid var(--vp-c-divider);
    border-radius: 4px;
    background: var(--vp-c-bg-soft);
    color: var(--vp-c-text-1);
    font: inherit;
    text-align: left;
    cursor: pointer;
}

.drawer[aria-pressed='true'] {
    border-color: var(--acc);
    background: var(--vp-c-bg-alt);
}

.drawer-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
}

.mark {
    width: 26px;
    height: 26px;
    fill: var(--vp-c-text-2);
    transition: fill 0.3s;
}

.drawer[aria-pressed='true'] .mark {
    fill: var(--acc);
}

.plate,
.chip {
    padding: 1px 8px;
    border-radius: 2px;
}

.plate {
    border: 1px solid var(--vp-c-border);
    color: var(--vp-c-text-2);
}

.drawer strong {
    font-size: 18px;
    font-weight: 600;
}

.drawer .use {
    font-size: 14px;
    line-height: 1.35;
    color: var(--vp-c-text-3);
}

.snippet {
    margin-top: 14px;
}

.snippet {
    font-family: var(--vp-font-family-base);
}

/* Undo the docs' phone-width bleed (negative margins, square corners): here the samples sit inside a column. */
.snippet :deep(div[class*='language-']),
.snippet :deep(.vp-code-group),
.snippet :deep(.vp-code-group .tabs) {
    margin: 0;
}

.snippet :deep(div[class*='language-']) {
    border-radius: 8px;
}

.snippet :deep(.vp-code-group .tabs) {
    border-radius: 8px 8px 0 0;
}

.snippet :deep(.vp-code-group div[class*='language-']) {
    border-radius: 0 0 8px 8px;
}

.more {
    display: inline-block;
    margin-top: 12px;
    color: var(--acc);
    font-weight: 600;
    text-decoration: none;
}

.files {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr);
    gap: 24px;
    align-items: start;
}

.files .snippet {
    margin-top: 0;
}

.file {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--vp-c-divider);
}

.file .ext {
    flex: none;
    width: 92px;
    color: var(--acc);
}

.file .what {
    flex: 1;
    font-size: 15px;
    color: var(--vp-c-text-2);
}

.chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
}

.chip {
    border: 1px solid var(--acc);
    color: var(--vp-c-text-1);
}

.colophon .body {
    padding-bottom: 96px;
}

.colophon p {
    max-width: 620px;
    margin: 0 0 14px;
    color: var(--vp-c-text-2);
}

.colophon b {
    font-weight: 600;
    color: var(--vp-c-text-1);
}

.fonte {
    font-style: italic;
    font-weight: 600;
    color: var(--fonte-molten);
}

.footer {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 22px;
    padding: 22px 32px 26px;
    border-top: 1px solid var(--vp-c-divider);
    background: var(--vp-c-bg-alt);
}

.footer-name {
    font-size: 18px;
    font-weight: 600;
}

.footer a {
    color: var(--acc);
    text-decoration: none;
}

.license {
    margin-left: auto;
    color: var(--vp-c-text-3);
}

@media (max-width: 720px) {
    .landing {
        --rail-height: 44px;
    }

    .sec {
        grid-template-columns: minmax(0, 1fr);
        padding: 0 16px;
    }

    .rail {
        z-index: 5;
        top: var(--vp-nav-height);
        height: var(--rail-height);
        flex-direction: row;
        align-items: baseline;
        gap: 10px;
        padding: 10px 0;
        background: var(--vp-c-bg);
    }

    .hero .rail {
        padding-top: 10px;
    }

    .rail .num {
        font-size: 22px;
    }

    .rail .name,
    .rail .deg {
        margin: 0;
    }

    .stage {
        top: calc(var(--vp-nav-height) + var(--rail-height));
        height: calc(100svh - var(--vp-nav-height) - var(--rail-height));
    }

    .body {
        padding: 24px 0 48px;
    }

    .drawers {
        grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    }

    .files,
    .features {
        grid-template-columns: minmax(0, 1fr);
    }

    .illustration {
        width: 48px;
        height: 48px;
    }

    .cells {
        grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .tray {
        gap: 18px;
    }

    .hint {
        margin-top: 18px;
    }

    .footer {
        gap: 16px;
        padding: 20px 16px 24px;
    }

    .license {
        width: 100%;
        margin-left: 0;
    }
}
</style>
