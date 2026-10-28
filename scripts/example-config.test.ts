import { cp, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { loadConfigFromFile } from 'vite-plus';
import { expect, it } from 'vite-plus/test';

it('loads example tasks before the plugin has been built', async () => {
    const root = await mkdtemp(resolve(import.meta.dirname, '../node_modules/.example-config-'));
    try {
        const plugin = join(root, 'node_modules/vite-svg-2-webfont');
        await mkdir(plugin, { recursive: true });
        await cp(resolve(import.meta.dirname, '../packages/vite-svg-2-webfont/package.json'), join(plugin, 'package.json'));
        await writeFile(join(root, 'package.json'), JSON.stringify({ type: 'module' }));
        const configFile = join(root, 'vite.config.ts');
        await cp(resolve(import.meta.dirname, '../packages/example/vite.config.ts'), configFile);
        const loaded = await loadConfigFromFile({ command: 'build', mode: 'production' }, configFile);
        expect(loaded).toMatchObject({ config: { run: { tasks: { build: { dependsOn: ['vite-svg-2-webfont#pack'] } } } } });
    } finally {
        await rm(root, { recursive: true, force: true });
    }
});
