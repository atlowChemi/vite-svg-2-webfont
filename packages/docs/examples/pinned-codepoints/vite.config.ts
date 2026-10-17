import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            // [!code focus:9]
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'pinned-icons',
            codepoints: {
                // [!code highlight:5]
                add: 0xe001,
                search: 0xe002,
                star: 0xe003,
            },
        }),
    ],
});
