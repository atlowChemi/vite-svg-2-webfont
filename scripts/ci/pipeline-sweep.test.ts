import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parse } from 'yaml';
import { expect, it } from 'vite-plus/test';

it('discovers new pipelines and rejects stale exclusions before execution', () => {
    const workflow = parse(readFileSync(new URL('../../.github/workflows/cache-audit.yaml', import.meta.url), 'utf8'));
    const script = workflow.jobs['select-work'].steps.find((step: { name?: string }) => step.name === 'Select finite pipelines').run;
    const { pipelines } = parse(readFileSync(new URL('../../pnpm-tasks.yaml', import.meta.url), 'utf8'));
    const directory = mkdtempSync(join(tmpdir(), 'pipeline-sweep-'));
    try {
        writeFileSync(join(directory, 'pnpm'), '#!/bin/sh\ncat "$PIPELINES_FIXTURE"\n', { mode: 0o755 });
        const fixture = join(directory, 'fixture.json');
        const env = { ...process.env, PATH: `${directory}:${process.env.PATH}`, PIPELINES_FIXTURE: fixture };
        writeFileSync(fixture, JSON.stringify({ ...pipelines, 'new-validation': ['new'] }));
        execFileSync('bash', ['-e', '-o', 'pipefail', '-c', script], { cwd: directory, env });
        const selected = readFileSync(join(directory, 'selected-pipelines.txt'), 'utf8').trim().split('\n');
        expect(selected).toContain('new-validation');
        expect(selected).toContain('docs');
        expect(selected).toContain('doctests');
        expect(selected).not.toContain('plugin-publish');
        expect(selected).not.toContain('docs-dev');

        delete pipelines['plugin-publish'];
        writeFileSync(fixture, JSON.stringify(pipelines));
        const stale = spawnSync('bash', ['-e', '-o', 'pipefail', '-c', script], { cwd: directory, env, encoding: 'utf8' });
        expect(stale.status).not.toBe(0);
        expect(stale.stderr).toContain('Unknown blocked pipelines: plugin-publish');
        expect(readFileSync(join(directory, 'selected-pipelines.txt'), 'utf8')).toBe('');
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});
