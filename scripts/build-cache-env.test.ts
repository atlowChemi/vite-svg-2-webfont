import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadConfigFromFile, resolveConfig, type UserConfig } from 'vite-plus';
import { expect, it, vi } from 'vite-plus/test';

// Docs examples need packed workspace artifacts; env-policy validation does not.
vi.mock('../packages/docs/.vitepress/examples', () => ({ examplePlugins: () => [] }));
vi.mock('../packages/docs/.vitepress/svgAuthoringFonts', () => ({ svgAuthoringFontPlugins: () => [] }));

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
