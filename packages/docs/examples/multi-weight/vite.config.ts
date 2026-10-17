import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        // [!code focus:11]
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, 'icons'),
            fontName: 'weighted-icons',
            // [!code highlight:5]
            variants: [
                { name: 'light', context: 'light', weight: 300 },
                { name: 'regular', context: 'regular', weight: 400, default: true },
                { name: 'bold', context: 'bold', weight: 700 },
            ],
            missingGlyphs: { behavior: 'fallback', variant: 'regular' }, // [!code highlight]
        }),
    ],
});
