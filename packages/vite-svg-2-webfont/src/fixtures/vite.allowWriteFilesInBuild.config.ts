import { resolve } from 'path';
import { defineConfig, type UserConfig } from 'vite';
import { viteSvgToWebfont } from '../../';

const webfontFolder = resolve(import.meta.dirname, 'webfont-test', 'svg');
const outputFolder = resolve(import.meta.dirname, 'webfont-test', 'artifacts');

const config: UserConfig = defineConfig({
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
export default config;
