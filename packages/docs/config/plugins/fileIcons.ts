import { icons } from '@iconify-json/vscode-icons';
import { encodeSvgForCss, getIconData, iconToHTML, iconToSVG } from '@iconify/utils';
import type { Plugin, ViteDevServer } from 'vite';

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
    `${labels
        .values()
        .map(label => prefix + selector(label))
        .reduce((list, next) => `${list},\n${next}`)} {\n    content: '';\n    --icon: ${svgUrl(name)};\n}\n`;

/** The stylesheet for these labels: one rule per icon, in a fixed order whatever order the labels came in. */
export function fileIconCss(labels: Iterable<string>): string {
    // oxlint-disable-next-line unicorn/no-array-sort -- The one array, just allocated by Array.from(); sorting a copy would add another.
    const sorted = Array.from(new Set(labels)).sort();
    // Grouped by the rule's own icon object, so labels a rule matches share one CSS rule; `undefined` holds the rest.
    return Map.groupBy(sorted, iconFor)
        .entries()
        .reduce((css, [icon, group]) => {
            if (!icon) return css;
            return (
                css +
                (typeof icon === 'string'
                    ? rule('', group, icon)
                    : rule('', group, icon.light) +
                      rule('html.dark ', group, icon.dark) +
                      `@media (prefers-color-scheme: dark) {\n${rule('html:not(.light) ', group, icon.dark)}}\n`)
            );
        }, '');
}

const ID = 'virtual:file-icons.css';
/** Labels no page renders: the Playground's output tabs exist only in the browser. */
export const browserLabels = new Set(['output.css', 'output.html', 'output.svg', 'output.ts']);
let labels = new Set<string>();
let server: ViteDevServer | undefined;

/** The build hands over every page's labels at once, before the stylesheet is loaded. */
export const setFileIconLabels = (rendered: Set<string>): void => {
    labels = rendered;
};

/**
 * The dev server adds each page's labels as it serves the page, so it never renders every page up front. A new label
 * invalidates the stylesheet; the browser revalidates it on the next page load and gets the new rules.
 */
export const addFileIconLabels = (rendered: Iterable<string>): void => {
    const known = labels.size;
    for (const label of rendered) labels.add(label);
    if (labels.size === known || !server) return;
    const { moduleGraph } = server.environments.client;
    const module = moduleGraph.getModuleById(`\0${ID}`);
    if (module) moduleGraph.invalidateModule(module);
};

export const fileIcons = (): Plugin => ({
    name: 'docs-file-icons',
    configureServer: dev => {
        server = dev;
    },
    resolveId: id => (id === ID ? `\0${ID}` : undefined),
    load: id => (id === `\0${ID}` ? fileIconCss(labels.union(browserLabels)) : undefined),
});
