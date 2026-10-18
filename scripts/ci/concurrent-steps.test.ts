import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vite-plus/test';
import { parse } from 'yaml';

type Step = {
    id?: string;
    name?: string;
    background?: boolean;
    wait?: string | string[];
    parallel?: Step[];
    run?: string;
    uses?: string;
    if?: string;
    with?: Record<string, unknown>;
};
type Workflow = { jobs: Record<string, { steps?: Step[] }> };
// Resolve from the repository root, independent of the test runner's cwd.
const rootWorkflows = new URL('../../.github/workflows/', import.meta.url);
const readWorkflow = (name: string): Workflow => parse(readFileSync(new URL(name, rootWorkflows), 'utf8'));

describe('concurrent workflow steps', () => {
    it('joins every background step and waits before consuming its outputs or outcome', () => {
        for (const file of readdirSync(rootWorkflows)) {
            for (const job of Object.values(readWorkflow(file).jobs)) {
                const active = new Set<string>();
                const ids = new Set<string>();
                // Expand groups into their equivalent background steps and implicit join.
                const steps = (job.steps ?? []).flatMap((step, group) => {
                    if (!step.parallel) return [step];
                    const children: Step[] = step.parallel.map((child, index) => ({ ...child, id: child.id ?? `parallel-${group}-${index}`, background: true }));
                    children.push({ wait: children.map(child => child.id!) });
                    return children;
                });
                for (const step of steps) {
                    for (const id of [step.wait ?? []].flat()) {
                        expect(active.delete(id), `${file}: wait for ${id}`).toBe(true);
                    }
                    for (const match of JSON.stringify(step).matchAll(/steps\.([\w-]+)\./g)) {
                        expect(active.has(match[1]!), `${file}: unjoined reference to ${match[1]}`).toBe(false);
                    }
                    if (step.id) {
                        assert.ok(!ids.has(step.id), `${file}: duplicate ${step.id}`);
                        ids.add(step.id);
                    }
                    if (step.background) {
                        // Hosted runner races observed in checkout/Codecov when uses overlap.
                        assert.equal(step.uses, undefined, `${file}: only run steps may execute in the background`);
                        assert.ok(step.id);
                        active.add(step.id);
                    }
                }
                expect([...active], `${file}: unjoined steps`).toEqual([]);
            }
        }
    });

    it('archives every JUnit upload with the same condition and report path', () => {
        for (const job of Object.values(readWorkflow('main.yaml').jobs)) {
            for (const step of job.steps ?? []) {
                if (step.uses !== './.github/actions/upload-test-results') continue;
                const archive = job.steps?.find(candidate => candidate.uses?.startsWith('actions/upload-artifact@') && candidate.with?.path === step.with?.files);
                expect(archive?.if).toBe(step.if);
                expect(archive?.with?.['if-no-files-found']).toBe('error');
                expect(archive?.with?.name).toBe(`test-results-${String(step.with?.name)}-\${{ github.run_attempt }}`);
            }
        }
    });

    it.each([false, true])('waits for every staged package and propagates failures (failure=%s)', fail => {
        const dir = mkdtempSync(join(tmpdir(), 'concurrent-staging-'));
        try {
            const script = readWorkflow('release.yaml').jobs['publish-native']!.steps!.find(step => step.name === 'Stage publish per-platform binary packages')!.run!;
            writeFileSync(join(dir, 'vp'), '#!/bin/bash\nif [[ "$*" == *broken* ]]; then exit 7; fi\nsleep 0.1\necho staged-ok\n');
            execFileSync('chmod', ['+x', join(dir, 'vp')]);
            const summary = join(dir, 'summary');
            const result = spawnSync('bash', ['-eo', 'pipefail', '-c', script], {
                env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, DIRS: `packages/ok\npackages/${fail ? 'broken' : 'also-ok'}`, GITHUB_STEP_SUMMARY: summary },
                encoding: 'utf8',
            });
            assert.equal(result.status, fail ? 1 : 0, result.stderr);
            expect(readFileSync(summary, 'utf8')).toContain('staged-ok');
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
});
