// The site's own Vite plugins. They build fonts with vite-svg-2-webfont, so `vite.config.ts` loads this module only once
// the plugin and its native module are built; task discovery runs before that, with no plugins at all.
import type { Plugin } from 'vite';
import { groupIconVitePlugin } from 'vitepress-plugin-group-icons';
import { examplePlugins } from './examples.ts';
import { siteIconFont } from './siteIconFont.ts';
import { svgAuthoringFontPlugins } from './svgAuthoringFonts.ts';

// The file icons beside code-group and example-tab labels.
const groupIcons = groupIconVitePlugin({
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
});

type Hook = (this: unknown, ...args: unknown[]) => unknown;
const handler = (hook: unknown) => (typeof hook === 'function' ? hook : (hook as { handler: Hook }).handler) as Hook;

/** The plugin collects labels from the Markdown it transforms; here it is handed the labels the pages rendered. */
export const tellGroupIcons = async (labels: string[]) => {
    await handler(groupIcons.transform).call({}, labels.map(label => `<label data-title="${label}">`).join(''), 'labels.md');
};

export const sitePlugins = (): Plugin[] => [groupIcons, siteIconFont(), ...svgAuthoringFontPlugins(), ...examplePlugins()];
