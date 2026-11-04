import { resolve } from 'node:path';
import type { Plugin } from 'vite';
import viteSvgToWebfont from '../../vite-svg-2-webfont/src/index.js';
import { siteIcons } from './theme/siteIcons';

/** The site's icon font, imported by the theme as `virtual:site-icons.css`. */
export function siteIconFont(): Plugin {
    return viteSvgToWebfont({
        context: resolve(import.meta.dirname, '..', 'site-icons'),
        fontName: 'fonte-site-icons',
        moduleId: 'site-icons.css',
        baseSelector: '.site-icon',
        classPrefix: 'site-icon-',
        codepoints: siteIcons,
        ligature: false,
        types: ['woff2'],
        // Inline, so the header's icons paint with the page instead of waiting out `font-display: block`.
        inline: true,
    });
}
