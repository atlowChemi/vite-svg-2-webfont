import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { it } from 'vitest';
import config from '../.github/renovate-config';

it('Renovate intents are repeatable and request patches for every published npm package', () => {
    assert.ok(config.postUpgradeTasks?.dataFileTemplate?.includes('{{{newDigest}}}'));
    const cwd = mkdtempSync(join(tmpdir(), 'renovate-intent-'));
    try {
        const data = join(cwd, 'updates');
        writeFileSync(data, 'renovate/example\nexample: 2.0.0 first-digest\n');
        const run = () =>
            execFileSync(process.execPath, [fileURLToPath(new URL('./renovate-intent.mjs', import.meta.url))], {
                cwd,
                env: { ...process.env, RENOVATE_POST_UPGRADE_COMMAND_DATA_FILE: data },
            });
        run();
        run();
        const intents = readdirSync(join(cwd, '.changeset'));
        assert.equal(intents.length, 1);
        const content = readFileSync(join(cwd, '.changeset', intents[0]!), 'utf8');
        for (const dir of ['webfont-generator', 'webfont-browser', 'vite-svg-2-webfont']) {
            const { name } = JSON.parse(readFileSync(new URL(`../packages/${dir}/package.json`, import.meta.url), 'utf8'));
            assert.ok(content.includes(`"${name}": patch`));
        }
        writeFileSync(data, 'renovate/example\nexample: 2.0.0 second-digest\n');
        run();
        assert.equal(readdirSync(join(cwd, '.changeset')).length, 2);
    } finally {
        rmSync(cwd, { recursive: true, force: true });
    }
});
