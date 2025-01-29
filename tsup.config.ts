import { defineConfig } from 'tsup';

export default defineConfig(options => ({
    entry: ['src/index.ts'],
    target: 'node18.0',
    dts: true,
    format: ['esm', 'cjs'],
    clean: !options.watch,
    minify: !options.watch,
}));
