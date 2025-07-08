import { defineConfig, type UserConfig, type UserConfigFn } from 'tsdown';

const config: UserConfig | UserConfigFn = defineConfig(options => ({
    format: ['esm', 'cjs'],
    clean: !options.watch,
    minify: !options.watch,
    outputOptions: {
        exports: 'named',
    },
}));

export default config;
