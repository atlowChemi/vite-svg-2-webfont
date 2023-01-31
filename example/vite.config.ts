import { resolve } from 'path';
import { defineConfig } from 'vite';
import { viteSvgToWebfont } from 'vite-svg-2-webfont';

const webfontFolder = resolve('./src/webfont');

// https://vitejs.dev/config/
export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            context: webfontFolder,
            htmlDest: resolve(webfontFolder, 'icons.ts'),
            htmlTemplate: resolve(webfontFolder, 'icons.ts.hbs'),
            fontName: 'exampleIcon',
            baseSelector: '.exIcon',
            generateFiles: 'html',
        }),
    ],
});
