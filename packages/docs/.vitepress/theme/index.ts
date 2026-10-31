import { h } from 'vue';
import type { Theme } from 'vitepress';
import DefaultTheme from 'vitepress/theme';
import FonteWordmark from './FonteWordmark.vue';
import Mermaid from './Mermaid.vue';
import '@fontsource/zilla-slab/600.css';
import '@fontsource/jetbrains-mono/400.css';
import 'virtual:group-icons.css';
import './custom.css';

const theme: Theme = {
    extends: DefaultTheme,

    // The wordmark goes in the nav title link, ahead of the subtitle (`themeConfig.siteTitle`).
    Layout: () => h(DefaultTheme.Layout, null, { 'nav-bar-title-before': () => h(FonteWordmark) }),

    enhanceApp({ app }) {
        app.component('Mermaid', Mermaid);
    },
};

export default theme;
