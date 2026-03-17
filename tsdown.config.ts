import { defineConfig, type UserConfig, type UserConfigFn } from 'tsdown';

const config: UserConfig | UserConfigFn = defineConfig(options => ({
    format: ['esm', 'cjs'],
    clean: !options.watch,
    minify: !options.watch,
    fixedExtension: false,
    nodeProtocol: true,
    deps: {
        onlyBundle: false,
        skipNodeModulesBundle: true,
    },
    outputOptions: {
        exports: 'named',
    },
}));

export default config;
