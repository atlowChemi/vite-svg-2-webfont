import { resolve } from 'node:path';
import { defineProject, type UserProjectConfigExport } from 'vite-plus';

const webfontFolder = resolve(import.meta.dirname, 'src', 'webfont');

let viteSvgToWebfont: Awaited<typeof import('../vite-svg-2-webfont/src')>['viteSvgToWebfont'] | undefined;
try {
    ({ viteSvgToWebfont } = await import('../vite-svg-2-webfont/src/index.js'));
} catch {
    // Ignore errors, as the native module might not be built yet, and we don't want to fail the config loading because of that.
}

// https://vitejs.dev/config/
const config: UserProjectConfigExport = defineProject({
    build: {
        assetsInlineLimit: 0,
    },
    plugins: viteSvgToWebfont
        ? [
              viteSvgToWebfont({
                  context: webfontFolder,
                  htmlDest: resolve(webfontFolder, 'icons.ts'),
                  htmlTemplate: resolve(webfontFolder, 'icons.ts.hbs'),
                  fontName: 'exampleIcon',
                  baseSelector: '.exIcon',
                  generateFiles: 'html',
              }),
          ]
        : [],
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

export default config;
