import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        // [!code focus:10]
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'pinned-icons',
            // [!code highlight:5]
            codepoints: {
                add: 0xe001,
                search: 0xe002,
                star: 0xe003,
            },
        }),
    ],
});
