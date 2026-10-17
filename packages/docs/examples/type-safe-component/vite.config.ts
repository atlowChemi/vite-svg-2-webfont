import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            // [!code focus:7]
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'typed-icons',
            generateFiles: 'html', // [!code highlight]
            htmlTemplate: resolve(import.meta.dirname, 'icons.ts.hbs'), // [!code highlight]
            htmlDest: resolve(import.meta.dirname, 'icons.ts'), // [!code highlight]
        }),
    ],
});
