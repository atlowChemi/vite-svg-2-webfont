import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            // [!code focus:6]
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'preloaded-icons',
            types: ['woff2', 'woff'],
            preloadFormats: ['woff2'], // [!code highlight]
        }),
    ],
});
