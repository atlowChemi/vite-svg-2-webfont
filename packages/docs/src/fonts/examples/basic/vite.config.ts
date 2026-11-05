import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        // [!code focus:3]
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, '../icons'), // [!code highlight]
        }),
    ],
});
