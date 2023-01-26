import { defineConfig } from 'tsup';

export default defineConfig(options => ({
    entry: ['src/index.ts'],
    target: 'node16',
    dts: true,
    format: ['esm', 'cjs'],
    clean: !options.watch,
    minify: !options.watch,
}));
