import { defineConfig, type MarkdownOptions } from 'vitepress';
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons';
import llmstxt from 'vitepress-plugin-llms';
import { codeThemes } from './codeThemes';
import { examplePlugins } from './examples';
import { svgAuthoringFontPlugins } from './svgAuthoringFonts';
import { iconCss } from './icon-css';

const repoName = 'vite-svg-2-webfont';
const repo = `https://github.com/atlowChemi/${repoName}`;
const docsUrl = 'https://fonte.atlow.dev/';
const socialImage = `${docsUrl}social-card.png`;
const siteName = 'fonte';
const siteSubtitle = 'SVG icons, cast into fonts';
const siteDescription = 'A Rust engine that casts SVG icons into fonts, with a CLI, Node and browser bindings, and a Vite plugin.';

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

const guideSidebar = {
    text: 'Guide',
    items: [
        { text: 'Choosing a Tool', link: '/guide/' },
        { text: 'The Engine', link: '/guide/engine' },
        { text: 'Under the Hood', link: '/guide/under-the-hood' },
        { text: 'Templates', link: '/guide/templates' },
        { text: 'SVG Authoring & Troubleshooting', link: '/guide/svg-authoring' },
        { text: 'Glossary', link: '/guide/glossary' },
    ],
};

const featuresSidebar = {
    text: 'Features',
    items: [
        { text: 'Engine Cache', link: '/guide/features/caching' },
        { text: 'Variants', link: '/guide/features/variants' },
        { text: 'Color Glyphs', link: '/guide/features/color-glyphs' },
        { text: 'Incremental Updates', link: '/guide/features/incremental' },
        { text: 'Codepoints and Ligatures', link: '/guide/features/codepoints-and-ligatures' },
    ],
};

const tools = [
    { text: 'Vite Plugin', link: '/vite/' },
    { text: 'Node.js', link: '/node/' },
    { text: 'Browser', link: '/browser/' },
    { text: 'CLI', link: '/cli/' },
    { text: 'Rust', link: '/rust/' },
];

// Every tool section lists its own pages first, then the shared guide and the other tools.
const toolSidebar = (text: string, items: { text: string; link: string }[]) => [{ text, items }, guideSidebar, featuresSidebar, { text: 'Tools', items: tools }];

export default defineConfig({
    title: siteName,
    description: siteDescription,
    cleanUrls: true,
    // Fragments included into other pages, not pages of their own.
    srcExclude: ['_shared/**'],
    head: [
        ['link', { rel: 'icon', href: '/logo.svg' }],
        ['meta', { name: 'theme-color', content: '#faf7f2', media: '(prefers-color-scheme: light)' }],
        ['meta', { name: 'theme-color', content: '#1c1a1f', media: '(prefers-color-scheme: dark)' }],
        ['meta', { property: 'og:type', content: 'website' }],
        ['meta', { property: 'og:site_name', content: siteName }],
        ['meta', { property: 'og:title', content: siteName }],
        ['meta', { property: 'og:description', content: siteDescription }],
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
        ['meta', { name: 'twitter:title', content: siteName }],
        ['meta', { name: 'twitter:description', content: siteDescription }],
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
        siteTitle: siteSubtitle,
        nav: [
            { text: 'Guide', link: '/guide/', activeMatch: '^/guide/' },
            { text: 'Tools', items: tools },
            { text: 'Playground', link: '/playground' },
            {
                text: 'Changelogs',
                items: [
                    { text: 'Vite Plugin', link: '/vite/changelog' },
                    { text: 'Node.js', link: '/node/changelog' },
                    { text: 'Browser', link: '/browser/changelog' },
                    { text: 'Rust Engine & CLI', link: '/rust/changelog' },
                ],
            },
        ],
        sidebar: {
            '/guide/': [guideSidebar, featuresSidebar, { text: 'Tools', items: tools }],
            '/vite/': toolSidebar('Vite Plugin', [
                { text: 'Getting Started', link: '/vite/' },
                { text: 'Usage', link: '/vite/usage' },
                { text: 'Examples', link: '/vite/examples' },
                { text: 'How It Works', link: '/vite/how-it-works' },
                { text: 'Configuration', link: '/vite/configuration' },
                { text: 'Public API', link: '/vite/public-api' },
                { text: 'Migration', link: '/vite/migration' },
                { text: 'Changelog', link: '/vite/changelog' },
            ]),
            '/node/': toolSidebar('Node.js', [
                { text: 'Getting Started', link: '/node/' },
                { text: 'Usage', link: '/node/usage' },
                { text: 'Examples', link: '/node/examples' },
                { text: 'Configuration', link: '/node/configuration' },
                { text: 'API', link: '/node/api' },
                { text: 'Changelog', link: '/node/changelog' },
            ]),
            '/browser/': toolSidebar('Browser', [
                { text: 'Getting Started', link: '/browser/' },
                { text: 'Usage', link: '/browser/usage' },
                { text: 'Changelog', link: '/browser/changelog' },
            ]),
            '/cli/': toolSidebar('CLI', [
                { text: 'Getting Started', link: '/cli/' },
                { text: 'Usage', link: '/cli/usage' },
                { text: 'Changelog', link: '/rust/changelog' },
            ]),
            '/rust/': toolSidebar('Rust', [
                { text: 'Getting Started', link: '/rust/' },
                { text: 'Usage', link: '/rust/usage' },
                { text: 'Changelog', link: '/rust/changelog' },
            ]),
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
