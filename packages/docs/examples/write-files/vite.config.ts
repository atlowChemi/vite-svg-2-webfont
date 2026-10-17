import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            // [!code focus:8]
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'written-icons',
            types: ['woff2', 'woff'],
            dest: resolve(import.meta.dirname, 'generated'), // [!code highlight]
            generateFiles: true, // [!code highlight]
            allowWriteFilesInBuild: true, // [!code highlight]
        }),
    ],
});
