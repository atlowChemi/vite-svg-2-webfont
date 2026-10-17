import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            // [!code focus:5]
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'ligature-icons',
            ligature: true, // [!code highlight]
        }),
    ],
});
