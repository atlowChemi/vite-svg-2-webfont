import { resolve } from 'node:path';
import type { Plugin } from 'vite';
import viteSvgToWebfont from '../../vite-svg-2-webfont/src/index.js';
import { svgAuthoringFonts } from './svgAuthoringCases';

const fixtures = resolve(import.meta.dirname, '..', 'svg-authoring-fixtures');
const allFontsModule = 'virtual:svg-authoring-fonts';

/** Every case becomes its own inline WOFF2 font, scoped by a per-case base selector and class prefix. */
export function svgAuthoringFontPlugins(): Plugin[] {
    const fonts = svgAuthoringFonts.map(({ id, dir = id, options }) =>
        viteSvgToWebfont({
            context: resolve(fixtures, dir),
            fontName: `sa-${id}`,
            moduleId: `svg-authoring-${id}.css`,
            baseSelector: `.sa-${id}`,
            classPrefix: `sa-${id}-`,
            types: ['woff2'],
            inline: true,
            ...options,
        }),
    );
    const imports = svgAuthoringFonts.map(({ id }) => `import 'virtual:svg-authoring-${id}.css';`).join('\n');
    const allFonts: Plugin = {
        name: 'svg-authoring-fonts',
        resolveId: id => (id === allFontsModule ? `\0${allFontsModule}` : undefined),
        load: id => (id === `\0${allFontsModule}` ? imports : undefined),
    };
    return [...fonts, allFonts];
}
