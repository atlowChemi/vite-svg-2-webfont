import { resolve } from 'node:path';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default {
    plugins: [
        // [!code focus:5]
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, '../icons'),
            fontName: 'typed-icons',
            tsModuleId: 'typed-icons', // [!code highlight]
        }),
    ],
};
