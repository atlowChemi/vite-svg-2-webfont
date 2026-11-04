import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parse } from 'yaml';
import { expect, it } from 'vite-plus/test';

it('discovers new pipelines and rejects stale exclusions before execution', () => {
    const workflow = parse(readFileSync(new URL('../../.github/workflows/cache-audit.yaml', import.meta.url), 'utf8'));
    const script = workflow.jobs.pipelines.steps.find((step: { name?: string }) => step.name === 'Select dormant pipelines').run;
    const { pipelines } = parse(readFileSync(new URL('../../pnpm-tasks.yaml', import.meta.url), 'utf8'));
    const directory = mkdtempSync(join(tmpdir(), 'dormant-pipelines-'));
    try {
        writeFileSync(join(directory, 'vp'), '#!/bin/sh\ncat "$PIPELINES_FIXTURE"\n', { mode: 0o755 });
        const fixture = join(directory, 'fixture.json');
        const env = { ...process.env, PATH: `${directory}:${process.env.PATH}`, PIPELINES_FIXTURE: fixture };
        writeFileSync(fixture, JSON.stringify({ ...pipelines, 'next-new-validation': ['next:new'] }));
        execFileSync('bash', ['-e', '-o', 'pipefail', '-c', script], { cwd: directory, env });
        const selected = readFileSync(join(directory, 'selected-pipelines.txt'), 'utf8').trim().split('\n');
        expect(selected).toContain('next-new-validation');
        expect(selected).toContain('next-docs');
        expect(selected).toContain('next-doctests');
        expect(selected).not.toContain('next-plugin-publish');
        expect(selected).not.toContain('next-docs-dev');

        delete pipelines['next-plugin-publish'];
        writeFileSync(fixture, JSON.stringify(pipelines));
        const stale = spawnSync('bash', ['-e', '-o', 'pipefail', '-c', script], { cwd: directory, env, encoding: 'utf8' });
        expect(stale.status).not.toBe(0);
        expect(stale.stderr).toContain('Unknown blocked pipelines: next-plugin-publish');
        expect(readFileSync(join(directory, 'selected-pipelines.txt'), 'utf8')).toBe('');
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});
