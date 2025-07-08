import { defineConfig, mergeConfig, type UserConfig } from 'vite';
import configBasic from './vite.basic.config';

const config: UserConfig = mergeConfig(
    configBasic,
    defineConfig({
        build: {
            assetsInlineLimit: 0,
        },
    }),
);

export default config;
