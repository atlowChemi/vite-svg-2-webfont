import { resolve } from 'path';
import { defineConfig } from 'vite';
import { viteSvgToWebfont } from '../../';

const webfontFolder = resolve(__dirname, './webfont-test/svg');

export default defineConfig({
    build: {
        assetsInlineLimit: 0,
    },
    plugins: [viteSvgToWebfont({ context: webfontFolder })],
});
