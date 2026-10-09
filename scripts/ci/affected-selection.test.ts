import { describe, expect, it } from 'vite-plus/test';
import { readFileSync } from 'node:fs';
import { parse } from 'yaml';
import { determineValidationScope, resolveSelection, packageNames, selectJobs, type Selection } from './affected-selection';
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
        for (const name of ['test-host', 'test-docker', 'test-vite-compat', 'platform-output']) expect(workflow.jobs[name].needs).toContain('build');
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
        expect(selection.jobs['platform-output']).toBe(false);
        expect(selection.nativeBuildScope).toBe('linux-x64');
        expect(selection.rustSuites).toEqual([]);
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: true });
    });
    it('runs only adapter Rust tests for adapter-only changes', () => {
        const selection = scenario(['packages/webfont-generator/native/lib.rs'], [packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root]);
        expect(selection.rustSuites).toEqual(['adapter']);
        expect(selection.nativeBuildScope).toBe('full');
        expect(selection.jobs['platform-output']).toBe(true);
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: true });
    });
    it('runs all Rust suites and native targets for engine changes', () => {
        const selection = scenario(
            ['crates/webfont-generator/src/lib.rs'],
            [packageNames.engine, packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root],
        );
        expect(selection.rustSuites).toEqual(['engine', 'cli', 'adapter']);
        expect(selection.nativeBuildScope).toBe('full');
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: true });
    });
    it('rejects narrowed platform prerequisites or missing selected Rust suites', () => {
        const selection = scenario(['Cargo.lock'], [packageNames.root]);
        expect(() => verifyRequiredChecks(results(selection), { ...selection, nativeBuildScope: 'linux-x64' })).toThrow('full native build scope');
        expect(() => verifyRequiredChecks(results(selection), { ...selection, rustSuites: ['adapter'] })).toThrow('Rust coverage suites');
        expect(() => verifyRequiredChecks(results(selection), { ...selection, nativeBuildScope: undefined } as unknown as Selection)).toThrow('native build scope');
    });
    it('propagates parity failures through Required checks and reuses the full build', () => {
        const selection = scenario(['scripts/fixtures/platform-output/curves.svg'], [packageNames.root]);
        expect(selection.jobs['platform-output']).toBe(true);
        expect(workflow.jobs['platform-output'].with['use-existing-build']).toBe(true);
        expect(() => verifyRequiredChecks({ ...results(selection), 'platform-output': { result: 'failure' } }, selection)).toThrow('platform-output: failure');
    });
    it('uses separate coverage flags and only the selected matrix suites', () => {
        const config = parse(readFileSync(new URL('../../codecov.yml', import.meta.url), 'utf8'));
        expect(config.flags['rust-tests']).toBeUndefined();
        for (const suite of ['engine', 'cli', 'adapter']) expect(config.flags[`${suite}-tests`].carryforward).toBe(true);
        const job = workflow.jobs['rust-coverage'];
        expect(job.strategy.matrix).toEqual({ suite: '${{ fromJSON(needs.affected-selection.outputs.selection).rustSuites }}' });
        const upload = job.steps.find((step: { name?: string }) => step.name === 'Upload Rust coverage');
        expect(upload.with.flags).toBe('${{ matrix.suite }}-tests');
    });
    it('keeps all native consumers supplied and defaults release builds to full', () => {
        const build = parse(readFileSync(new URL('../../.github/workflows/build-native.yaml', import.meta.url), 'utf8'));
        const release = parse(readFileSync(new URL('../../.github/workflows/release.yaml', import.meta.url), 'utf8'));
        expect(build.on.workflow_call.inputs.scope.default).toBe('full');
        expect(workflow.jobs.build.with.scope).toBe('${{ fromJSON(needs.affected-selection.outputs.selection).nativeBuildScope }}');
        const expression: string = build.jobs.build.strategy.matrix.target;
        expect(expression).toContain("inputs.scope == 'linux-x64'");
        const [linux = [], full = []] = [...expression.matchAll(/'(\[[^']+\])'/g)].map(match => JSON.parse(match[1]!) as string[]);
        const download = workflow.jobs['test-vite-compat'].steps.find((step: { name?: string }) => step.name === 'Download binding');
        expect(linux.map(target => `bindings-${target}`)).toEqual([download.with.name]);
        for (const { target } of workflow.jobs['test-host'].strategy.matrix.settings) expect(full).toContain(target);
        for (const arch of workflow.jobs['test-docker'].strategy.matrix.arch) expect(full).toContain(`${arch}-unknown-linux-musl`);
        expect(full).toHaveLength(9);
        const releaseBuild = Object.values(release.jobs).find((job: unknown) => (job as { uses?: string }).uses === './.github/workflows/build-native.yaml') as {
            with?: { scope?: string };
        };
        expect(releaseBuild).toBeDefined();
        expect(releaseBuild.with?.scope ?? 'full').toBe('full');
    });
    it('reserves release matrix artifacts for adapter publication', () => {
        const release = parse(readFileSync(new URL('../../.github/workflows/release.yaml', import.meta.url), 'utf8'));
        expect(release.jobs['build-native'].if).toBe("needs.release-please.outputs.native-released == 'true'");
        expect(release.jobs['publish-native'].needs).toEqual(['release-please', 'build-native']);
        expect(release.jobs['publish-plugin'].needs).toEqual(['release-please']);
        for (const [job, output] of [
            ['publish-plugin', 'plugin-released'],
            ['publish-native', 'native-released'],
            ['publish-crate', 'engine-released'],
        ] as const) {
            expect(release.jobs[job].if).toBe(`needs.release-please.outputs.${output} == 'true'`);
        }
    });
    it('runs script tests for script changes, not every full package validation', () => {
        expect(scenario(['scripts/ci/affected-selection.ts'], [packageNames.root]).jobs['test-scripts']).toBe(true);
        expect(scenario(['pnpm-lock.yaml'], [packageNames.root]).jobs['test-scripts']).toBe(false);
    });
    it.each(['Cargo.lock', 'pnpm-lock.yaml', 'package.json', 'Cargo.toml', 'unknown/file'])('broadens shared changes: %s', file => {
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
    it('falls back for rewritten push history and empty selections', () => {
        expect(resolveSelection(base, () => 'c'.repeat(40), true).full).toBe(true);
        expect(() => scenario(['packages/new/a.ts'], ['new-package'])).toThrow('Unknown affected packages: new-package');
        expect(scenario(['packages/docs/a.md'], []).full).toBe(true);
    });
    it('uses affected selection on ordinary pushes', () => {
        const selection = resolveSelection(
            base,
            (_command, args) => {
                if (args[0] === 'merge-base') return base;
                if (args[0] === 'diff') return 'packages/docs/guide.md\0';
                return JSON.stringify([{ name: packageNames.docs }]);
            },
            true,
        );
        expect(selection.full).toBe(false);
        expect(selection.packages).toEqual([packageNames.docs]);
        expect(selection.jobs['test-scripts']).toBe(false);
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

describe('validation scope policy', () => {
    it.each(['packages/webfont-generator/package.json', 'packages/webfont-generator/Cargo.toml'])('keeps adapter manifest changes downstream: %s', file => {
        const affected = [packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root];
        expect(determineValidationScope([file], new Set(affected)).full).toBe(false);
        expect(scenario([file], affected).rustSuites).toEqual(['adapter']);
    });
    it.each([{ names: [] }, { names: [packageNames.root] }])('broadens changed files with an incomplete selection: $names', ({ names }) => {
        expect(determineValidationScope(['packages/docs/a.md'], new Set(names)).full).toBe(true);
    });
    it('permits an empty selection when no files changed', () => {
        expect(determineValidationScope([], new Set()).full).toBe(false);
    });
    it('rejects unknown packages even when shared changes would select everything', () => {
        expect(() => determineValidationScope(['pnpm-lock.yaml'], new Set(['new-package']))).toThrow('Update the affected-selection package and job mapping');
    });
});
