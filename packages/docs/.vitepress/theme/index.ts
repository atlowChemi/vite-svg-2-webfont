import { h } from 'vue';
import type { Theme } from 'vitepress';
import DefaultTheme from 'vitepress/theme';
import FonteWordmark from './FonteWordmark.vue';
import FonteLanding from './landing/FonteLanding.vue';
import Mermaid from './Mermaid.vue';
import PageEyebrow from './PageEyebrow.vue';
import '@fontsource/zilla-slab/600.css';
import '@fontsource/jetbrains-mono/400.css';
import 'virtual:group-icons.css';
import './custom.css';

const theme: Theme = {
    extends: DefaultTheme,

    // The wordmark goes in the nav title link, ahead of the subtitle (`themeConfig.siteTitle`); the eyebrow above each doc page.
    Layout: () => h(DefaultTheme.Layout, null, { 'nav-bar-title-before': () => h(FonteWordmark), 'doc-before': () => h(PageEyebrow) }),

    enhanceApp({ app }) {
        app.component('Mermaid', Mermaid);
        app.component('FonteLanding', FonteLanding);
    },
};

export default theme;
