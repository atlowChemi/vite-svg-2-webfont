import { resolve } from 'path';
import { defineConfig, type UserConfig } from 'vite';
import { viteSvgToWebfont } from '../../';

const webfontFolder = resolve(import.meta.dirname, 'webfont-test', 'svg');

const config: UserConfig = defineConfig({
    build: {
        assetsInlineLimit: 0,
    },
    plugins: [
        viteSvgToWebfont({
            context: webfontFolder,
            types: ['woff2', 'ttf'],
            preloadFormats: ['woff2', 'woff'],
        }),
    ],
});

export default config;
