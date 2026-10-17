import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        // [!code focus:6]
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'ui-icons', // [!code highlight]
            classPrefix: 'ui-icon-', // [!code highlight]
            baseSelector: '.ui-icon', // [!code highlight]
        }),
    ],
});
