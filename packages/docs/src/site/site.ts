// The site's identity and navigation, shared by the build and the shell.

export interface NavItem {
    text: string;
    link: string;
}
export interface NavGroup {
    text: string;
    items: NavItem[];
}

export const repo = 'https://github.com/atlowChemi/vite-svg-2-webfont';

export const site = {
    name: 'fonte',
    subtitle: 'SVG icons, cast into fonts',
    description: 'A Rust engine that casts SVG icons into fonts, with a CLI, Node and browser bindings, and a Vite plugin.',
    github: repo,
    editPattern: `${repo}/edit/main/packages/docs/src/pages/:path`,
};

const npm = (name: string) => `https://www.npmjs.com/package/${name}`;
const crate = 'https://crates.io/crates/webfont-generator';
export const tools = [
    { key: 'vite', plate: 'Plugin', name: 'Vite', package: 'vite-svg-2-webfont', registry: npm('vite-svg-2-webfont'), registryName: 'npm' },
    { key: 'node', plate: 'npm', name: 'Node.js', package: '@atlowchemi/webfont-generator', registry: npm('@atlowchemi/webfont-generator'), registryName: 'npm' },
    {
        key: 'browser',
        plate: 'Wasm',
        name: 'Browser',
        package: '@atlowchemi/webfont-browser-generator',
        registry: npm('@atlowchemi/webfont-browser-generator'),
        registryName: 'npm',
    },
    { key: 'cli', plate: 'Binary', name: 'CLI', package: 'webfont-generator --features cli', registry: crate, registryName: 'crates.io' },
    { key: 'rust', plate: 'Crate', name: 'Rust', package: 'webfont-generator', registry: crate, registryName: 'crates.io' },
] as const;
export type Tool = (typeof tools)[number];

const guide: NavGroup = {
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

const features: NavGroup = {
    text: 'Features',
    items: [
        { text: 'Engine Cache', link: '/guide/features/caching' },
        { text: 'Variants', link: '/guide/features/variants' },
        { text: 'Color Glyphs', link: '/guide/features/color-glyphs' },
        { text: 'Incremental Updates', link: '/guide/features/incremental' },
        { text: 'Codepoints and Ligatures', link: '/guide/features/codepoints-and-ligatures' },
    ],
};

/** Each section's page groups: the guide has two shelves, a tool its own pages. */
export const sections: Record<string, NavGroup[]> = {
    guide: [guide, features],
    vite: [
        {
            text: 'Vite Plugin',
            items: [
                { text: 'Getting Started', link: '/vite/' },
                { text: 'Usage', link: '/vite/usage' },
                { text: 'Examples', link: '/vite/examples' },
                { text: 'How It Works', link: '/vite/how-it-works' },
                { text: 'Configuration', link: '/vite/configuration' },
                { text: 'Public API', link: '/vite/public-api' },
                { text: 'Migration', link: '/vite/migration' },
                { text: 'Changelog', link: '/vite/changelog' },
            ],
        },
    ],
    node: [
        {
            text: 'Node.js',
            items: [
                { text: 'Getting Started', link: '/node/' },
                { text: 'Usage', link: '/node/usage' },
                { text: 'Examples', link: '/node/examples' },
                { text: 'Configuration', link: '/node/configuration' },
                { text: 'API', link: '/node/api' },
                { text: 'Changelog', link: '/node/changelog' },
            ],
        },
    ],
    browser: [
        {
            text: 'Browser',
            items: [
                { text: 'Getting Started', link: '/browser/' },
                { text: 'Usage', link: '/browser/usage' },
                { text: 'Examples', link: '/browser/examples' },
                { text: 'Configuration', link: '/browser/configuration' },
                { text: 'API', link: '/browser/api' },
                { text: 'Changelog', link: '/browser/changelog' },
            ],
        },
    ],
    cli: [
        {
            text: 'CLI',
            items: [
                { text: 'Getting Started', link: '/cli/' },
                { text: 'Usage', link: '/cli/usage' },
                { text: 'Examples', link: '/cli/examples' },
                { text: 'Configuration', link: '/cli/configuration' },
                { text: 'Changelog', link: '/rust/changelog' },
            ],
        },
    ],
    rust: [
        {
            text: 'Rust',
            items: [
                { text: 'Getting Started', link: '/rust/' },
                { text: 'Usage', link: '/rust/usage' },
                { text: 'Examples', link: '/rust/examples' },
                { text: 'Configuration', link: '/rust/configuration' },
                { text: 'API', link: '/rust/api' },
                { text: 'Changelog', link: '/rust/changelog' },
            ],
        },
    ],
};

/** A route without `.html`, `/index` or a trailing slash, so links and pages compare equal. */
export const normalize = (path: string) => path.replace(/(\.html|\/index)$/, '').replace(/\/$/, '') || '/';
