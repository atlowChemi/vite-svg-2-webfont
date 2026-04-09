import { resolve } from 'path';
import { defineConfig } from 'vite-plus';
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
    run: {
        tasks: {
            dev: {
                command: 'vp dev',
                dependsOn: ['vite-svg-2-webfont#pack'],
            },
            build: {
                command: 'vp build',
                dependsOn: ['vite-svg-2-webfont#pack'],
            },
            preview: {
                command: 'vp preview',
                dependsOn: ['build'],
            },
        },
    },
});
