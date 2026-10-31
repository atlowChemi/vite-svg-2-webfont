import type { Theme } from 'vitepress';
import DefaultTheme from 'vitepress/theme';
import Mermaid from './Mermaid.vue';
import '@fontsource/zilla-slab/600.css';
import '@fontsource/jetbrains-mono/400.css';
import 'virtual:group-icons.css';
import './custom.css';

const theme: Theme = {
    extends: DefaultTheme,

    enhanceApp({ app }) {
        app.component('Mermaid', Mermaid);
    },
};

export default theme;
