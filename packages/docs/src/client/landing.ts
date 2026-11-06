// The landing page. Its markup is the first frame; this pours the tray into the T as the reader scrolls through the hero,
// docks the wordmark into the top bar, cools the page toward the footer, and switches the tool drawers.
// It brings every landing component's module, and the class names it queries and toggles.
import landingStyles from '../components/landing/Landing.module.css';
import sectionStyles from '../components/landing/Section.module.css';
import heroStyles from '../components/landing/Hero.module.css';
import flyerStyles from '../components/landing/Flyer.module.css';
import drawerStyles from '../components/landing/Drawers.module.css';
import '../components/landing/Features.module.css';
import '../components/landing/Outputs.module.css';
import '../components/landing/Footer.module.css';
import '../components/landing/Colophon.module.css';
import { EMPTY, FULL, degrees, wave } from '../components/landing/pour.ts';

const html = document.documentElement;

// Drawers: every snippet and guide link is in the markup; the pressed drawer shows its own.
const drawerButtons = [...document.querySelectorAll<HTMLButtonElement>(`.${drawerStyles.drawer}[data-drawer]`)];
for (const button of drawerButtons) {
    button.addEventListener('click', () => {
        const key = button.dataset.drawer!;
        for (const other of drawerButtons) other.setAttribute('aria-pressed', String(other === button));
        for (const el of document.querySelectorAll<HTMLElement>('[data-drawer-panel], [data-drawer-link]')) {
            el.hidden = (el.dataset.drawerPanel ?? el.dataset.drawerLink) !== key;
        }
    });
}

type Rgb = [number, number, number];
// Accent temperatures, hot to cold. Light mode uses darker stops so the accent keeps 4.5:1 on ash, the footer's background;
// cold is the muted text color.
const heat: Record<'dark' | 'light', { hot: Rgb; ember: Rgb; cold: Rgb; ink: Rgb }> = {
    dark: { hot: [255, 106, 19], ember: [255, 176, 46], cold: [138, 130, 144], ink: [243, 236, 226] },
    light: { hot: [180, 65, 10], ember: [148, 84, 0], cold: [107, 99, 112], ink: [28, 26, 31] },
};
const clamp = (value: number) => Math.min(1, Math.max(0, value));
const mix = (a: Rgb, b: Rgb, t: number) => `rgb(${a.map((v, i) => Math.round(v + (b[i]! - v) * t)).join(' ')})`;

const root = document.querySelector<HTMLElement>(`.${landingStyles.landing}`);
const hero = document.querySelector<HTMLElement>(`.${heroStyles.hero}`);
const stage = document.querySelector<HTMLElement>(`.${heroStyles.stage}`);
const placeholder = document.querySelector<HTMLElement>(`.${heroStyles.placeholder}`);
const flyer = document.querySelector<HTMLElement>(`.${flyerStyles.flyer}`);
const trayItems = [...document.querySelectorAll<HTMLElement>(`.${heroStyles.trayItem}`)];
const hint = document.querySelector<HTMLElement>(`.${heroStyles.hint}`);
const result = document.querySelector<HTMLElement>(`.${heroStyles.result}`);
const rails = [...document.querySelectorAll<HTMLElement>(`.${sectionStyles.deg}`)];
const molten = flyer?.querySelector<SVGPathElement>('[data-part="molten"]');
const surface = flyer?.querySelector<SVGPathElement>('[data-part="surface"]');

type Dock = { y0: number; y1: number; fx: number; fy: number; tx: number; ty: number; k: number };
let dock: Dock | undefined;
const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
const scrollDriven = CSS.supports('animation-timeline: scroll()');
let ready = false;
let level = EMPTY;
let target = EMPTY;
let agitation = 0;
let phase = 0;
let landed = 0;
let shownDegrees = degrees(1);

// Where the hero wordmark sits while the stage is pinned, where the top bar keeps its slot, and the scroll span between.
function measure() {
    const navMark = document.querySelector<SVGElement>('[data-top-wordmark] svg');
    if (!navMark || !flyer || !hero || !stage || !placeholder) return;
    const stickTop = parseFloat(getComputedStyle(stage).top);
    const heroTop = hero.getBoundingClientRect().top + scrollY;
    const y0 = heroTop - stickTop;
    const y1 = heroTop + hero.offsetHeight - stickTop - stage.offsetHeight;
    const nav = navMark.getBoundingClientRect();
    dock = {
        y0,
        y1,
        fx: stage.getBoundingClientRect().left + placeholder.offsetLeft,
        fy: stickTop + placeholder.offsetTop,
        tx: nav.left,
        ty: nav.top,
        k: nav.height / placeholder.offsetHeight,
    };
    const vars = { '--from-x': dock.fx, '--from-y': dock.fy, '--to-x': dock.tx, '--to-y': dock.ty, '--dock-a': y0 + (y1 - y0) * 0.84, '--dock-b': y1 };
    for (const [name, value] of Object.entries(vars)) flyer.style.setProperty(name, `${value}px`);
    flyer.style.setProperty('--to-k', String(dock.k));
    if (ready) return;
    // Once the flyer knows where it flies, it takes over from the placeholder and the top bar's wordmark.
    ready = true;
    flyer.classList.add(flyerStyles.ready!);
    placeholder.classList.add(heroStyles.docked!);
    html.classList.add('landing-docked');
    // Two frames: the hidden top bar wordmark must be painted before its fade transition is switched on.
    requestAnimationFrame(() => requestAnimationFrame(() => html.classList.add('landing-fades')));
}

function update() {
    if (!dock || !flyer || !stage) return;
    const progress = reduce ? 1 : clamp((scrollY - dock.y0) / (dock.y1 - dock.y0));
    const colors = heat[html.classList.contains('dark') ? 'dark' : 'light'];

    // Each icon falls into the mouth of the T, heating up on the way, and raises the metal when it lands.
    const stageBox = stage.getBoundingClientRect();
    const mold = flyer.querySelector('[data-part="mold"]')!.getBoundingClientRect();
    const mouthX = mold.left + mold.width / 2 - stageBox.left;
    const mouthY = mold.top + mold.height * 0.22 - stageBox.top;
    let filled = 0;
    let count = 0;
    for (const [i, item] of trayItems.entries()) {
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
    target = EMPTY - ((EMPTY - FULL) / trayItems.length) * filled;

    if (hint) hint.style.opacity = String(clamp(1 - progress / 0.04));
    if (result) {
        const shown = clamp((progress - 0.62) / 0.14);
        result.style.opacity = String(shown);
        result.style.transform = `translateY(${(1 - shown) * 16}px)`;
        result.inert = shown < 0.5;
    }

    if (!scrollDriven) {
        let d = clamp((scrollY - (dock.y0 + (dock.y1 - dock.y0) * 0.84)) / ((dock.y1 - dock.y0) * 0.16));
        if (reduce) d = d >= 1 ? 1 : 0;
        const lerp = (a: number, b: number) => a + (b - a) * d;
        flyer.style.transform = `translate(${lerp(dock.fx, dock.tx)}px, ${lerp(dock.fy, dock.ty)}px) scale(${lerp(1, dock.k)})`;
        flyer.style.setProperty('--mold-width', String(lerp(1.4, 2.4)));
    }

    // The page cools once the cast is done: molten through ember to cold metal at the footer.
    const maxScroll = html.scrollHeight - innerHeight;
    const t = scrollY <= dock.y1 ? 1 : clamp(1 - (scrollY - dock.y1) / Math.max(1, maxScroll - dock.y1));
    root?.style.setProperty('--acc', t > 0.5 ? mix(colors.ember, colors.hot, (t - 0.5) / 0.5) : mix(colors.cold, colors.ember, t / 0.5));
    const label = degrees(t);
    if (label !== shownDegrees) {
        shownDegrees = label;
        for (const rail of rails) rail.textContent = label;
    }
}

// The metal eases toward its target level and its surface keeps moving; with reduced motion, one still frame.
function frame() {
    level += (target - level) * 0.12;
    agitation *= 0.97;
    phase += 0.035;
    const d = wave(level, (level < EMPTY - 1 ? 1.3 : 0) + agitation, phase);
    surface?.setAttribute('d', d);
    molten?.setAttribute('d', `${d} L100 100 L0 100 Z`);
    if (!reduce) requestAnimationFrame(frame);
}

function relayout() {
    measure();
    update();
}

if (root) {
    addEventListener('scroll', update, { passive: true });
    addEventListener('resize', relayout);
    void document.fonts.ready.then(relayout);
    // The theme toggle flips `dark` on <html>; the accent's stops follow it.
    new MutationObserver(update).observe(html, { attributes: true, attributeFilter: ['class'] });
    relayout();
    if (reduce) level = target;
    frame();
}
