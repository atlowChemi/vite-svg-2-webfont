import { resolve } from 'node:path';
import { defineProject, type UserProjectConfigExport } from 'vite-plus';
import { viteSvgToWebfont } from 'vite-svg-2-webfont';

const webfontFolder = resolve(import.meta.dirname, 'src', 'webfont');

// https://vitejs.dev/config/
const config: UserProjectConfigExport = defineProject({
    build: {
        assetsInlineLimit: 0,
    },
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

export default config;
