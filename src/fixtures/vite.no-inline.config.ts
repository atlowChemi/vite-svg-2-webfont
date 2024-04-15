import { defineConfig, mergeConfig } from 'vite';
import configBasic from './vite.basic.config';

export default mergeConfig(
    configBasic,
    defineConfig({
        build: {
            assetsInlineLimit: 0,
        },
    }),
);
