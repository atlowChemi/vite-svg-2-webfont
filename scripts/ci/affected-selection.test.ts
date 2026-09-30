import { describe, expect, it } from 'vite-plus/test';
import { readFileSync } from 'node:fs';
import { parse } from 'yaml';
import { resolveSelection, packageNames, selectJobs, type Selection } from './affected-selection';
import { verifyRequiredChecks } from './required-checks';

const base = 'b'.repeat(40);
const workflow = parse(readFileSync(new URL('../../.github/workflows/main.yaml', import.meta.url), 'utf8'));
function scenario(files: string[], packages: string[], output = JSON.stringify(packages.map(name => ({ name })))): Selection {
    return resolveSelection(base, (_command, args) => {
        if (args[0] === 'merge-base') return base;
        if (args[0] === 'diff') return files.join('\0');
        return output;
    });
}
const results = (selection: Selection) => ({
    'affected-selection': { result: 'success' },
    ...Object.fromEntries(Object.entries(selection.jobs).map(([name, selected]) => [name, { result: selected ? 'success' : 'skipped' }])),
});

describe('affected CI', () => {
    it('aligns decisions and prerequisites with workflow jobs', () => {
        const jobs = selectJobs(Object.values(packageNames), [], true);
        expect(Object.keys(jobs).toSorted()).toEqual(
            Object.keys(workflow.jobs)
                .filter(name => !['affected-selection', 'required-checks'].includes(name))
                .toSorted(),
        );
        expect(workflow.jobs['required-checks'].needs.toSorted()).toEqual(['affected-selection', ...Object.keys(jobs)].toSorted());
        for (const name of Object.keys(jobs).filter(job => job !== 'ci')) expect(workflow.jobs[name].if).toContain(`.jobs.${name}`);
        for (const name of ['test-host', 'test-docker', 'test-vite-compat']) expect(workflow.jobs[name].needs).toContain('build');
    });
    it('selects only shared checks and docs for docs edits', () => {
        const selection = scenario(['packages/docs/guide.md'], [packageNames.docs]);
        expect(
            Object.entries(selection.jobs)
                .filter(([, value]) => value)
                .map(([name]) => name),
        ).toEqual(['ci', 'docs']);
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: false });
    });
    it('provides plugin native artifacts without refreshing Rust coverage', () => {
        const selection = scenario(['packages/vite-svg-2-webfont/src/index.ts'], [packageNames.plugin, packageNames.example]);
        expect(selection.jobs.build).toBe(true);
        expect(selection.jobs['rust-coverage']).toBe(false);
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: true });
    });
    it('runs script tests for script changes, not every full package validation', () => {
        expect(scenario(['scripts/ci/affected-selection.ts'], [packageNames.root]).jobs['test-scripts']).toBe(true);
        expect(scenario(['pnpm-lock.yaml'], [packageNames.root]).jobs['test-scripts']).toBe(false);
    });
    it.each(['Cargo.lock', 'packages/docs/package.json', 'crates/webfont-generator/Cargo.toml', 'unknown/file'])('broadens shared or graph changes: %s', file => {
        expect(scenario([file], [packageNames.root]).full).toBe(true);
    });
    it.each(['not JSON', '{}', '[{}]'])('falls back for invalid pnpm output: %s', output => {
        expect(Object.values(scenario(['packages/docs/a.md'], [], output).jobs).every(Boolean)).toBe(true);
    });
    it.each([undefined, '0'.repeat(40), '--invalid-ref'])('falls back for invalid history: %s', revision => {
        expect(
            resolveSelection(revision, () => {
                throw new Error('Unavailable');
            }).full,
        ).toBe(true);
    });
    it('falls back for rewritten push history and unknown packages', () => {
        expect(resolveSelection(base, () => 'c'.repeat(40), true).full).toBe(true);
        expect(scenario(['packages/new/a.ts'], ['new-package']).full).toBe(true);
        expect(scenario(['packages/docs/a.md'], []).full).toBe(true);
    });
    it.each(['failure', 'cancelled', 'skipped', undefined])('rejects unsuccessful selected jobs: %s', result => {
        const selection = scenario(['packages/docs/guide.md'], [packageNames.docs]);
        expect(() => verifyRequiredChecks({ ...results(selection), docs: { result } }, selection)).toThrow('docs:');
    });
    it('rejects missing decisions, failed selection and missing prerequisites', () => {
        const selection = scenario([], []);
        expect(() => verifyRequiredChecks({}, selection)).toThrow('Affected selection');
        expect(() => verifyRequiredChecks(results(selection), null)).toThrow('Missing or invalid');
        expect(() => verifyRequiredChecks({ ...results(selection), extra: { result: 'skipped' } }, selection)).toThrow('job lists differ');
        selection.jobs['test-vite-compat'] = true;
        expect(() => verifyRequiredChecks(results(selection), selection)).toThrow('native build');
    });
    it('requires successful uploads and explicit coverage finalization', () => {
        const config = parse(readFileSync(new URL('../../codecov.yml', import.meta.url), 'utf8'));
        expect(config.codecov.notify.manual_trigger).toBe(true);
        expect(Object.values(config.flags).every(flag => (flag as { carryforward: boolean }).carryforward)).toBe(true);
        for (const job of ['test-scripts', 'rust-coverage', 'native-coverage', 'test-vite-compat']) {
            const uploads = workflow.jobs[job].steps.filter((step: { uses?: string }) => step.uses?.startsWith('codecov/codecov-action'));
            expect(uploads.length).toBeGreaterThan(0);
            for (const step of uploads) expect(step.with.fail_ci_if_error).toBe(true);
        }
        expect(workflow.jobs['required-checks'].steps.some((step: { with?: { run_command?: string } }) => step.with?.run_command === 'send-notifications')).toBe(true);
    });
    it('finalizes script coverage and rejects failed script jobs', () => {
        const selection = scenario([], []);
        selection.jobs['test-scripts'] = true;
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: true });
        expect(() => verifyRequiredChecks({ ...results(selection), 'test-scripts': { result: 'failure' } }, selection)).toThrow('test-scripts: failure');
    });
});
