import { h } from 'vue';
import { useData } from 'vitepress';
import type { Theme } from 'vitepress';
import DefaultTheme from 'vitepress/theme';
import FonteWordmark from './FonteWordmark.vue';
import FonteLanding from './landing/FonteLanding.vue';
import Mermaid from './Mermaid.vue';
import PageEyebrow from './PageEyebrow.vue';
import SketchLayout from './sketch/SketchLayout.vue';
import '@fontsource/zilla-slab/600.css';
import '@fontsource/jetbrains-mono/400.css';
import 'virtual:group-icons.css';
import './custom.css';

// Review aid: `?layout=classic` shows the current redesign instead of the sketch for the rest of the tab's session,
// and `?layout=sketch` switches back.
function classic(): boolean {
    if (typeof window === 'undefined') return false;
    const requested = new URLSearchParams(location.search).get('layout');
    if (requested) sessionStorage.setItem('fonte-layout', requested);
    return sessionStorage.getItem('fonte-layout') === 'classic';
}

const theme: Theme = {
    extends: DefaultTheme,

    // The wordmark goes in the nav title link, ahead of the subtitle (`themeConfig.siteTitle`); the eyebrow above each doc page.
    // Layout sketch: doc pages use their own shell; the landing page and the playground keep the default theme's.
    Layout: () => {
        const { frontmatter } = useData();
        if (frontmatter.value.layout !== 'page' && !classic()) return h(SketchLayout);
        return h(DefaultTheme.Layout, null, { 'nav-bar-title-before': () => h(FonteWordmark), 'doc-before': () => h(PageEyebrow) });
    },

    enhanceApp({ app }) {
        app.component('Mermaid', Mermaid);
        app.component('FonteLanding', FonteLanding);
    },
};

export default theme;
