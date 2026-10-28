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
    with?: { tool?: string };
};
type Workflow = { jobs: Record<string, { steps?: Step[] }> };
const rootWorkflows = new URL('../../.github/workflows/', import.meta.url);
const readWorkflow = (name: string): Workflow => parse(readFileSync(new URL(name, rootWorkflows), 'utf8'));
const installer = (uses: string): Step => ({ name: 'Install cargo-xwin', uses, with: { tool: 'cargo-xwin' } });

function auditActionOverlap(file: string, step: Step, active: Set<string>) {
    // Runner #4641: only the audited Linux shell installer may overlap LLVM.
    // Renovate may change the pin, but never the action identity or full-SHA requirement.
    const shellInstaller =
        file === 'build-native.yaml' && step.name === 'Install cargo-xwin' && step.with?.tool === 'cargo-xwin' && /^taiki-e\/install-action@[a-fA-F0-9]{40}$/.test(step.uses ?? '');
    if (step.uses) assert.deepEqual([...active], shellInstaller ? ['llvm'] : [], `${file}: action overlaps background work`);
}

describe('concurrent workflow steps', () => {
    it('joins shell work before unaudited actions or references to background results', () => {
        for (const file of readdirSync(rootWorkflows)) {
            for (const job of Object.values(readWorkflow(file).jobs)) {
                const active = new Set<string>();
                const ids = new Set<string>();
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
                    auditActionOverlap(file, step, active);
                    for (const match of JSON.stringify(step).matchAll(/steps\.([\w-]+)\./g)) {
                        expect(active.has(match[1]!), `${file}: unjoined reference to ${match[1]}`).toBe(false);
                    }
                    if (step.id) {
                        assert.ok(!ids.has(step.id), `${file}: duplicate ${step.id}`);
                        ids.add(step.id);
                    }
                    if (step.background) {
                        assert.ok(step.run && !step.uses, `${file}: only shell steps may run in the background`);
                        assert.ok(step.id);
                        active.add(step.id);
                    }
                }
                expect([...active], `${file}: unjoined steps`).toEqual([]);
            }
        }
    });

    it.each(['8cca0918f8471c33515202b73f56ab27f2bed283', '1e29709c0ec63babbe8f266397a465f29a825f91', 'ABCDEF0123'.repeat(4)])(
        'accepts any full commit pin for the audited installer: %s',
        sha => {
            expect(() => auditActionOverlap('build-native.yaml', installer(`taiki-e/install-action@${sha}`), new Set(['llvm']))).not.toThrow();
        },
    );
    it.each(['v2', 'main', '1e29709', 'a'.repeat(39), 'a'.repeat(41), 'g'.repeat(40), `${'a'.repeat(40)}/extra`, ''])('rejects unpinned or malformed refs: %s', ref => {
        expect(() => auditActionOverlap('build-native.yaml', installer(`taiki-e/install-action@${ref}`), new Set(['llvm']))).toThrow('action overlaps background work');
    });
    it.each([`other/install-action@${'a'.repeat(40)}`, `taiki-e/other-action@${'a'.repeat(40)}`, `taiki-e/install-action/subpath@${'a'.repeat(40)}`])(
        'rejects other action identities: %s',
        uses => {
            expect(() => auditActionOverlap('build-native.yaml', installer(uses), new Set(['llvm']))).toThrow('action overlaps background work');
        },
    );
    it('limits the exception to the audited workflow, step, tool and background work', () => {
        const step = installer(`taiki-e/install-action@${'a'.repeat(40)}`);
        expect(() => auditActionOverlap('main.yaml', step, new Set(['llvm']))).toThrow('action overlaps background work');
        expect(() => auditActionOverlap('build-native.yaml', { ...step, name: 'Install something else' }, new Set(['llvm']))).toThrow('action overlaps background work');
        expect(() => auditActionOverlap('build-native.yaml', { ...step, with: { tool: 'cargo-zigbuild' } }, new Set(['llvm']))).toThrow('action overlaps background work');
        expect(() => auditActionOverlap('build-native.yaml', step, new Set(['other']))).toThrow('action overlaps background work');
        expect(() => auditActionOverlap('build-native.yaml', step, new Set(['llvm', 'other']))).toThrow('action overlaps background work');
        expect(() => auditActionOverlap('main.yaml', { uses: `actions/checkout@${'a'.repeat(40)}` }, new Set())).not.toThrow();
    });

    it.each([false, true])('waits for staged packages and propagates failures (failure=%s)', fail => {
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
            expect(readFileSync(summary, 'utf8').match(/staged-ok/g)).toHaveLength(fail ? 1 : 2);
        } finally {
            rmSync(dir, { recursive: true, force: true });
        }
    });
});
