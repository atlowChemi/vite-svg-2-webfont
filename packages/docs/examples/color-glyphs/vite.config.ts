import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        // [!code focus:6]
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'color-icons',
            types: ['woff2'], // [!code highlight]
            colorGlyphs: ['logo', 'flag'], // [!code highlight]
        }),
    ],
});
