import { h } from 'vue';
import { useData } from 'vitepress';
import type { Theme } from 'vitepress';
import DefaultTheme from 'vitepress/theme';
import FonteNotFound from './FonteNotFound.vue';
import FonteWordmark from './FonteWordmark.vue';
import FonteLanding from './landing/FonteLanding.vue';
import Mermaid from './Mermaid.vue';
import DocLayout from './doc/DocLayout.vue';
import '@fontsource/zilla-slab/600.css';
import '@fontsource/jetbrains-mono/400.css';
import 'virtual:group-icons.css';
import 'virtual:site-icons.css';
import './custom.css';

const theme: Theme = {
    extends: DefaultTheme,

    // Doc pages use their own shell. The landing page, the playground and the 404 page (`layout: page`) keep the default
    // theme's, with the wordmark in the nav title link ahead of the subtitle (`themeConfig.siteTitle`), and the 404 page
    // shows the type case.
    Layout: () => {
        const { frontmatter } = useData();
        if (frontmatter.value.layout !== 'page') return h(DocLayout);
        return h(DefaultTheme.Layout, null, { 'nav-bar-title-before': () => h(FonteWordmark), 'not-found': () => h(FonteNotFound) });
    },

    enhanceApp({ app }) {
        app.component('Mermaid', Mermaid);
        app.component('FonteLanding', FonteLanding);
    },
};

export default theme;
