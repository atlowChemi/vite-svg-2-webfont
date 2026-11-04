import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadConfigFromFile, resolveConfig, type UserConfig } from 'vite';
import { expect, it, vi } from 'vitest';
import { build } from 'tsdown';

// Docs examples need packed workspace artifacts; env-policy validation does not.
vi.mock('../packages/docs/.vitepress/examples', () => ({ examplePlugins: () => [] }));
vi.mock('../packages/docs/.vitepress/svgAuthoringFonts', () => ({ svgAuthoringFontPlugins: () => [] }));

it.each(['webfont-browser', 'vite-svg-2-webfont'])('keeps ignored environment values out of %s bundles', async name => {
    const root = mkdtempSync(join(tmpdir(), 'cached-bundle-env-'));
    try {
        const { default: config } = await import(fileURLToPath(new URL(`../packages/${name}/tsdown.config.ts`, import.meta.url)));
        writeFileSync(join(root, 'index.ts'), 'export const probe = process.env.TSDOWN_CACHE_CONTRACT_PROBE;');
        writeFileSync(join(root, 'package.json'), '{"type":"module"}');
        const options = {
            ...config,
            cwd: root,
            entry: ['index.ts'],
            format: ['esm'],
            outDir: 'dist',
            fixedExtension: false,
            dts: false,
            plugins: [],
            copy: [],
            logLevel: 'silent',
        };
        await build(options);
        const baseline = readFileSync(join(root, 'dist/index.js'), 'utf8');
        writeFileSync(join(root, '.env'), 'TSDOWN_CACHE_CONTRACT_PROBE=untracked-file\n');
        vi.stubEnv('TSDOWN_CACHE_CONTRACT_PROBE', 'untracked-process');
        await build(options);
        expect(readFileSync(join(root, 'dist/index.js'), 'utf8')).toBe(baseline);
    } finally {
        vi.unstubAllEnvs();
        rmSync(root, { recursive: true, force: true });
    }
});

it.each([
    ['packages/webfont-browser/vite.config.ts', 'package'],
    ['packages/vite-svg-2-webfont/vite.config.ts', 'package'],
    ['packages/docs/.vitepress/config.ts', 'docs'],
    ['packages/vite-svg-2-webfont/vite.config.ts', 'test'],
])('keeps ignored dotenv values out of %s (%s)', async (file, kind) => {
    const root = mkdtempSync(join(tmpdir(), 'cached-build-env-'));
    try {
        writeFileSync(join(root, '.env.production.local'), 'VITE_CACHE_CONTRACT_PROBE=untracked\n');
        const baseline = await resolveConfig({ root, configFile: false, logLevel: 'silent' }, 'build', 'production');
        expect(baseline.env.VITE_CACHE_CONTRACT_PROBE).toBe('untracked');
        // Runtime loading keeps the docs project's types out of unrelated package typechecks.
        const docsConfig = fileURLToPath(new URL('../packages/docs/.vitepress/config.ts', import.meta.url));
        const loaded =
            kind === 'docs'
                ? { config: (await import(docsConfig)).default as { vite: UserConfig } }
                : await loadConfigFromFile({ command: 'build', mode: 'production' }, fileURLToPath(new URL(`../${file}`, import.meta.url)));
        expect(loaded).not.toBeNull();
        const config =
            kind === 'docs'
                ? (loaded!.config as { vite: UserConfig }).vite
                : kind === 'test'
                  ? (loaded!.config as { test: { projects: UserConfig[] } }).test.projects[0]!
                  : (loaded!.config as UserConfig);
        // Resolve the actual env policy without executing unrelated package/docs plugins.
        const resolved = await resolveConfig({ root, configFile: false, envDir: config.envDir, logLevel: 'silent' }, 'build', 'production');
        expect(resolved.env.VITE_CACHE_CONTRACT_PROBE).toBeUndefined();
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});
