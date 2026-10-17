import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            // [!code focus:5]
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'custom-template-icons',
            cssTemplate: resolve(import.meta.dirname, 'icons.css.hbs'), // [!code highlight]
        }),
    ],
});
