import { resolve } from 'path';
import { defineConfig } from 'vite';
import { viteSvgToWebfont } from '../../';

const webfontFolder = resolve(__dirname, './webfont-test/svg');
const outputFolder = resolve(__dirname, './webfont-test/artifacts');

export default defineConfig({
    build: {
        assetsInlineLimit: 0,
    },
    plugins: [
        viteSvgToWebfont({
            dest: outputFolder,
            generateFiles: true,
            context: webfontFolder,
            allowWriteFilesInBuild: true,
            fontName: 'allowWriteFilesInBuild-test',
        }),
    ],
});
