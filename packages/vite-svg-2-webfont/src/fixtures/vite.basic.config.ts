import { resolve } from 'path';
import { defineConfig, type UserConfig } from 'vite';
import { viteSvgToWebfont } from '../../';

const webfontFolder = resolve(__dirname, './webfont-test/svg');

const config: UserConfig = defineConfig({
    plugins: [viteSvgToWebfont({ context: webfontFolder })],
});

export default config;
