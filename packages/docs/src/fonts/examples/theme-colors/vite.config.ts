import { resolve } from 'node:path';
import { defineConfig } from 'vite-plus';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, 'icons'),
            fontName: 'theme-icons',
            types: ['woff2'],
            colorGlyphs: true,
            colorSlots: { secondary: '#059669' }, // [!code highlight]
            cssTemplate: resolve(import.meta.dirname, 'icons.css.hbs'),
        }),
    ],
});
