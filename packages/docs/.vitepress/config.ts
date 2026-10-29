import { defineConfig, type MarkdownOptions } from 'vitepress';
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons';
import llmstxt from 'vitepress-plugin-llms';
import { codeThemes } from './codeThemes';
import { examplePlugins } from './examples';
import { svgAuthoringFontPlugins } from './svgAuthoringFonts';
import { iconCss } from './icon-css';

const repoName = 'vite-svg-2-webfont';
const repo = `https://github.com/atlowChemi/${repoName}`;
const base = process.env.GITHUB_ACTIONS ? `/${repoName}/` : '/';
const docsUrl = `https://atlowChemi.github.io/${repoName}/`;
const socialImage = `${docsUrl}social-card.png`;

const markdown: MarkdownOptions = {
    theme: codeThemes,
    config(md) {
        md.use(groupIconMdPlugin);
        const defaultRenderer = md.renderer.rules.fence;

        if (!defaultRenderer) {
            throw new Error('defaultRenderer is undefined');
        }

        md.renderer.rules.fence = (tokens, index, options, env, slf) => {
            const token = tokens[index];
            if (!token) return defaultRenderer(tokens, index, options, env, slf);
            const language = token.info.trim();

            if (language.startsWith('mermaid')) {
                const key = index;
                return `
                    <Suspense>
                    <template #default>
                    <Mermaid id="mermaid-${key}" graph="${encodeURIComponent(token.content)}"></Mermaid>
                    </template>
                        <!-- loading state via #fallback slot -->
                        <template #fallback>
                        Loading...
                        </template>
                    </Suspense>
                `;
            }
            return defaultRenderer(tokens, index, options, env, slf);
        };
    },
};

const guidesSidebar = {
    text: 'Guides',
    items: [
        { text: 'Which Package Do I Need?', link: '/which-package' },
        { text: 'SVG Authoring & Troubleshooting', link: '/svg-authoring' },
        { text: 'Glossary', link: '/glossary' },
    ],
};

export default defineConfig({
    title: repoName,
    description: 'A Vite plugin that generates webfonts from SVG icons.',
    base,
    cleanUrls: true,
    head: [
        ['link', { rel: 'icon', href: `${base}logo.svg` }],
        ['meta', { name: 'theme-color', content: '#646cff' }],
        ['meta', { property: 'og:type', content: 'website' }],
        ['meta', { property: 'og:site_name', content: repoName }],
        ['meta', { property: 'og:title', content: repoName }],
        [
            'meta',
            {
                property: 'og:description',
                content: 'A Vite plugin that generates webfonts from SVG icons.',
            },
        ],
        ['meta', { property: 'og:url', content: docsUrl }],
        ['meta', { property: 'og:image', content: socialImage }],
        [
            'meta',
            {
                property: 'og:image:alt',
                content: 'vite-svg-2-webfont social preview',
            },
        ],
        ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
        ['meta', { name: 'twitter:title', content: repoName }],
        [
            'meta',
            {
                name: 'twitter:description',
                content: 'A Vite plugin that generates webfonts from SVG icons.',
            },
        ],
        ['meta', { name: 'twitter:image', content: socialImage }],
    ],
    markdown,
    vite: {
        css: { postcss: iconCss },
        plugins: [
            groupIconVitePlugin({
                customIcon: {
                    '.tsx': 'vscode-icons:file-type-reactts',
                    '.component.ts': 'vscode-icons:file-type-angular',
                    '.svg': 'vscode-icons:file-type-svg',
                    '.hbs': 'vscode-icons:file-type-handlebars',
                    '.handlebars': 'vscode-icons:file-type-handlebars',
                    handlebars: 'vscode-icons:file-type-handlebars',
                    'generated css': 'vscode-icons:file-type-css',
                    'generated ts': 'vscode-icons:file-type-typescript',
                },
                // Tab labels rendered by Vue components, which the Markdown scan does not see.
                defaultLabels: ['Generated CSS', 'Generated TS', 'output.css', 'output.html', 'output.svg', 'output.ts'],
            }),
            llmstxt(),
            ...svgAuthoringFontPlugins(),
            ...examplePlugins(markdown),
        ],
        resolve: {
            alias: {
                mermaid: 'mermaid/dist/mermaid.esm.mjs',
            },
        },
    },
    themeConfig: {
        logo: '/logo.svg',
        siteTitle: repoName,
        nav: [
            { text: 'Guide', link: '/getting-started' },
            { text: 'Which Package?', link: '/which-package' },
            { text: 'Playground', link: '/playground' },
            {
                text: 'Webfont Generator',
                items: [
                    { text: 'Overview', link: '/webfont-generator/' },
                    { text: 'Node.js', link: '/webfont-generator/node' },
                    { text: 'Rust', link: '/webfont-generator/rust' },
                    { text: 'Browser', link: '/webfont-generator/browser' },
                    { text: 'CLI', link: '/webfont-generator/cli' },
                    { text: 'Under the Hood', link: '/webfont-generator/under-the-hood' },
                    { text: 'npm Adapter Changelog', link: '/webfont-generator/changelog' },
                    { text: 'Rust Engine Changelog', link: '/webfont-generator/engine-changelog' },
                ],
            },
            { text: 'Changelog', link: '/changelog' },
        ],
        sidebar: {
            '/': [
                {
                    text: 'Vite Plugin',
                    items: [
                        { text: 'Getting Started', link: '/getting-started' },
                        { text: 'Usage', link: '/usage' },
                        { text: 'Examples', link: '/examples' },
                        { text: 'How It Works', link: '/how-it-works' },
                        { text: 'Configuration', link: '/configuration' },
                        { text: 'Public API', link: '/public-api' },
                        { text: 'Migration', link: '/migration' },
                    ],
                },
                guidesSidebar,
            ],
            '/webfont-generator/': [
                {
                    text: 'Webfont Generator',
                    items: [
                        { text: 'Overview', link: '/webfont-generator/' },
                        { text: 'Node.js', link: '/webfont-generator/node' },
                        { text: 'Rust', link: '/webfont-generator/rust' },
                        { text: 'Browser', link: '/webfont-generator/browser' },
                        { text: 'CLI', link: '/webfont-generator/cli' },
                        { text: 'Templates', link: '/webfont-generator/templates' },
                        { text: 'Under the Hood', link: '/webfont-generator/under-the-hood' },
                        { text: 'npm Adapter Changelog', link: '/webfont-generator/changelog' },
                        { text: 'Rust Engine Changelog', link: '/webfont-generator/engine-changelog' },
                    ],
                },
                guidesSidebar,
            ],
        },
        search: {
            provider: 'local',
        },
        socialLinks: [
            { icon: 'rust', link: `https://crates.io/crates/webfont-generator` },
            { icon: 'npm', link: `https://www.npmjs.com/package/${repoName}` },
            { icon: 'github', link: repo },
        ],
        outline: [2, 3],
        footer: {
            message: 'Released under the MIT License.',
            copyright: 'Copyright © Chemi Atlow',
        },
        editLink: {
            pattern: `${repo}/edit/main/packages/docs/:path`,
        },
    },
});
