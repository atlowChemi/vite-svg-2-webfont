import { resolve } from 'path';
import { defineConfig } from 'vite';
import { viteSvgToWebfont } from '../';

const webfontFolder = resolve(__dirname, './webfont-test/svg');

export default defineConfig({
    plugins: [
        viteSvgToWebfont({ context: webfontFolder }),
    ],
});
