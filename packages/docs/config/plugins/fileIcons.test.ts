import { describe, expect, it } from 'vite-plus/test';
import { browserLabels, fileIconCss, iconFor } from './fileIcons.ts';

const light = (name: string) => ({ light: `file-type-light-${name}`, dark: `file-type-${name}` });

// Every label the site rendered when this replaced vitepress-plugin-group-icons, with the icon that plugin chose.
const labels: [string, ReturnType<typeof iconFor>][] = [
    ['.github/workflows/icons.yaml', light('yaml-official')],
    ['build.rs', light('rust')],
    ['bun', 'file-type-bun'],
    ['Cargo.toml', light('toml')],
    ['dist/index.html', 'file-type-html'],
    ['generate-icons.ts', 'file-type-typescript'],
    ['Generated CSS', 'file-type-css'],
    ['Generated TS', 'file-type-typescript'],
    ['icon-list.hbs', 'file-type-handlebars'],
    ['icon-properties.css.hbs', 'file-type-handlebars'],
    ['icon.component.ts', 'file-type-angular'],
    ['Icon.svelte', 'file-type-svelte'],
    ['Icon.tsx', 'file-type-reactts'],
    ['Icon.vue', 'file-type-vue'],
    ['icons.css.hbs', 'file-type-handlebars'],
    ['icons.ts', 'file-type-typescript'],
    ['icons.webfont.json', light('json')],
    ['index.html', 'file-type-html'],
    ['main.ts', 'file-type-typescript'],
    ['my-icons.css', 'file-type-css'],
    ['my-icons.ts', 'file-type-typescript'],
    ['Node.js', light('js')],
    ['npm', 'file-type-npm'],
    ['output.css', 'file-type-css'],
    ['output.html', 'file-type-html'],
    ['output.svg', 'file-type-svg'],
    ['output.ts', 'file-type-typescript'],
    ['package.json', 'file-type-node'],
    ['pnpm', light('pnpm')],
    ['scripts/build-icons.ts', 'file-type-typescript'],
    ['server.ts', 'file-type-typescript'],
    ['src/components/Icon.tsx', 'file-type-reactts'],
    ['src/components/Icon.vue', 'file-type-vue'],
    ['src/main.rs', light('rust')],
    ['styles/app.scss', 'file-type-scss'],
    ['tsconfig.json', 'file-type-tsconfig'],
    ['Vite', light('vite')],
    ['vite-env.d.ts', light('vite')],
    ['vite.config.ts', light('vite')],
    ['vite.config.v6.ts', light('vite')],
    ['yarn', 'file-type-yarn'],
    ...[
        'Browser',
        'CLI',
        'Components',
        'CSS',
        'Dockerfile',
        'Download',
        'generated/',
        'GitHub Actions',
        'GitLab CI',
        'HTML',
        'icons/',
        'Output',
        'Rust',
        'Shared types',
        'Type setup',
        'Upload',
        'Usage',
        'Use on the page',
    ].map(label => [label, undefined] as [string, undefined]),
];

describe('file icons', () => {
    it.each(labels)('%s', (label, icon) => {
        expect(iconFor(label)).toEqual(icon);
    });

    it('writes the same stylesheet whatever order the labels come in', () => {
        const names = labels.map(([label]) => label);
        expect(fileIconCss(names)).toBe(fileIconCss(names.toReversed()));
    });

    it('gives a themed icon dark-mode rules: for a dark pick, and for a dark system with no pick', () => {
        const css = fileIconCss(['Cargo.toml']);
        expect(css).toContain(`[data-title='Cargo.toml']::before`);
        expect(css).toContain(`html.dark [data-title='Cargo.toml']::before`);
        expect(css).toMatch(/@media \(prefers-color-scheme: dark\) \{\nhtml:not\(\.light\) \[data-title='Cargo\.toml'\]::before/);
    });

    it('escapes quotes in labels', () => {
        expect(fileIconCss(["it's.ts"])).toContain(String.raw`[data-title='it\'s.ts']::before`);
    });

    it('has an icon for every label only the browser renders', () => {
        expect([...browserLabels].filter(label => !iconFor(label))).toEqual([]);
    });

    it('writes nothing for labels without an icon', () => {
        expect(fileIconCss(['Output'])).toBe('');
    });
});
