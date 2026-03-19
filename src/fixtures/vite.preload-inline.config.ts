import { resolve } from 'path';
import { defineConfig, type UserConfig } from 'vite';
import { viteSvgToWebfont } from '../../';

const webfontFolder = resolve(__dirname, './webfont-test/svg');

const config: UserConfig = defineConfig({
    build: {
        assetsInlineLimit: 0,
    },
    plugins: [
        viteSvgToWebfont({
            context: webfontFolder,
            inline: true,
            preloadFormats: ['woff2'],
            types: ['woff2'],
        }),
    ],
});

export default config;
