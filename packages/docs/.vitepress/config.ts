import { defineConfig } from 'vitepress';
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons';
import llmstxt from 'vitepress-plugin-llms';

const repoName = 'vite-svg-2-webfont';
const repo = `https://github.com/atlowChemi/${repoName}`;
const base = process.env.GITHUB_ACTIONS ? `/${repoName}/` : '/';
const docsUrl = `https://atlowChemi.github.io/${repoName}/`;
const socialImage = `${docsUrl}social-card.png`;

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
    markdown: {
        config(md) {
            md.use(groupIconMdPlugin);
        },
    },
    vite: {
        plugins: [groupIconVitePlugin(), llmstxt()],
        resolve: { preserveSymlinks: true },
    } as never,
    themeConfig: {
        logo: '/logo.svg',
        siteTitle: repoName,
        nav: [
            { text: 'Guide', link: '/getting-started' },
            { text: 'Configuration', link: '/configuration' },
            { text: 'Public API', link: '/public-api' },
            { text: 'Changelog', link: '/changelog' },
        ],
        sidebar: [
            {
                items: [
                    { text: 'Getting Started', link: '/getting-started' },
                    { text: 'Usage', link: '/usage' },
                    { text: 'Configuration', link: '/configuration' },
                    { text: 'Public API', link: '/public-api' },
                ],
            },
        ],
        search: {
            provider: 'local',
        },
        socialLinks: [
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
