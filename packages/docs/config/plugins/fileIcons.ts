import { icons } from '@iconify-json/vscode-icons';
import { encodeSvgForCss, getIconData, iconToHTML, iconToSVG } from '@iconify/utils';
import type { Plugin } from 'vite';

// File icons beside code-group tabs and code-block titles. The pages render every label as `data-title`; this module
// serves `virtual:file-icons.css`, which sets `--icon` on each label's `::before` (the tabs and titles size and draw it).

/** A VS Code Icons name, or a light and a dark one. */
type Icon = string | { light: string; dark: string };

/** An icon with a lighter variant: `file-type-x` in dark mode, `file-type-light-x` in light mode. */
const themed = (name: string): Icon => ({ light: `file-type-light-${name}`, dark: `file-type-${name}` });

/** Labels and their icons; the first match wins. Labels without one (`Output`, `CLI`) get no icon. */
const rules: [RegExp, Icon][] = [
    [/(?:^|[^a-z])vite(?:[^a-z]|$)/i, themed('vite')],
    [/^npm$/, 'file-type-npm'],
    [/^yarn$/, 'file-type-yarn'],
    [/^bun$/, 'file-type-bun'],
    [/^pnpm$/, themed('pnpm')],
    [/\.component\.ts$/, 'file-type-angular'],
    [/\.tsx$/, 'file-type-reactts'],
    [/\.vue$/, 'file-type-vue'],
    [/\.svelte$/, 'file-type-svelte'],
    [/(?:^|\/)package\.json$/, 'file-type-node'],
    [/(?:^|\/)tsconfig\.json$/, 'file-type-tsconfig'],
    [/\.json$/, themed('json')],
    [/\.(?:hbs|handlebars)$/, 'file-type-handlebars'],
    [/\.ts$|^Generated TS$/, 'file-type-typescript'],
    [/\.css$|^Generated CSS$/, 'file-type-css'],
    [/\.scss$/, 'file-type-scss'],
    [/\.html$/, 'file-type-html'],
    [/\.svg$/, 'file-type-svg'],
    [/\.rs$/, themed('rust')],
    [/\.toml$/, themed('toml')],
    [/\.ya?ml$/, themed('yaml-official')],
    [/\.js$/, themed('js')],
];

export const iconFor = (label: string): Icon | undefined => rules.find(([match]) => match.test(label))?.[1];

function svgUrl(name: string): string {
    const data = getIconData(icons, name);
    if (!data) throw new Error(`No VS Code icon named ${name}`);
    const { attributes, body } = iconToSVG(data);
    return `url("data:image/svg+xml,${encodeSvgForCss(iconToHTML(body, attributes))}")`;
}

const selector = (label: string) => `[data-title='${label.replace(/['\\]/g, '\\$&')}']::before`;
const rule = (prefix: string, labels: string[], name: string) =>
    `${labels.map(label => prefix + selector(label)).join(',\n')} {\n    content: '';\n    --icon: ${svgUrl(name)};\n}\n`;

/** The stylesheet for these labels: one rule per icon, in a fixed order whatever order the labels came in. */
export function fileIconCss(labels: Iterable<string>): string {
    const byIcon = new Map<string, { icon: Icon; labels: string[] }>();
    for (const label of [...new Set(labels)].toSorted()) {
        const icon = iconFor(label);
        if (!icon) continue;
        const key = JSON.stringify(icon);
        const entry = byIcon.get(key) ?? { icon, labels: [] };
        entry.labels.push(label);
        byIcon.set(key, entry);
    }
    return [...byIcon.values()]
        .map(({ icon, labels: group }) => (typeof icon === 'string' ? rule('', group, icon) : rule('', group, icon.light) + rule('html.dark ', group, icon.dark)))
        .join('');
}

const ID = 'virtual:file-icons.css';
let labels: string[] = [];

/** Hands over the labels the pages render, before the stylesheet is loaded. */
export const setFileIconLabels = (rendered: string[]): void => {
    labels = rendered;
};

export const fileIcons = (): Plugin => ({
    name: 'docs-file-icons',
    resolveId: id => (id === ID ? `\0${ID}` : undefined),
    load: id => (id === `\0${ID}` ? fileIconCss(labels) : undefined),
});
