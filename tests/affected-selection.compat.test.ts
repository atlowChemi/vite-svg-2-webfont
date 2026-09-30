import { describe, expect, it } from 'vite-plus/test';
import { readFileSync } from 'node:fs';
import { parse } from 'yaml';
import { resolveSelection, packageNames, parseChangedFiles, parsePackages, selectJobs, selectionSummary } from '../scripts/ci/affected-selection.ts';
import { verifyRequiredChecks } from '../scripts/ci/required-checks.ts';

const selected = (selection: ReturnType<typeof selectJobs>) =>
    Object.entries(selection.jobs)
        .filter(([, job]) => job.selected)
        .map(([name]) => name);
const head = 'a'.repeat(40);
const base = 'b'.repeat(40);

describe('affected CI policy', () => {
    it('covers every validation job and Rust suite in the workflow', () => {
        const workflow = parse(readFileSync(new URL('../.github/workflows/main.yaml', import.meta.url), 'utf8'));
        const selection = selectJobs([], [], 'Full validation');
        expect(Object.keys(selection.jobs).toSorted()).toEqual(
            Object.keys(workflow.jobs)
                .filter(name => !['affected-selection', 'required-checks'].includes(name))
                .toSorted(),
        );
        expect(selection.rustSuites.toSorted()).toEqual(workflow.jobs['rust-coverage'].strategy.matrix.include.map((entry: { suite: string }) => entry.suite).toSorted());
        expect(workflow.jobs['required-checks'].needs.toSorted()).toEqual(['affected-selection', ...Object.keys(selection.jobs)].toSorted());
        for (const job of Object.keys(selection.jobs).filter(name => name !== 'ci')) {
            expect(workflow.jobs[job].needs).toContain('affected-selection');
            expect(workflow.jobs[job].if).toContain(`.jobs.${job}.selected`);
        }
        for (const job of ['test-host', 'test-docker', 'test-vite-compat']) expect(workflow.jobs[job].needs).toContain('build');
    });
    it('keeps docs-only validation narrow', () => {
        expect(selected(selectJobs(['packages/docs/getting-started.md'], [packageNames.docs]))).toEqual(['ci', 'docs']);
    });

    it('provides native artifacts for plugin tests without claiming engine source changed', () => {
        const result = selectJobs(['packages/vite-svg-2-webfont/src/index.ts'], [packageNames.plugin, packageNames.example]);
        expect(selected(result)).toEqual(['ci', 'build', 'test-browser', 'test-vite-compat']);
        expect(result.rustSuites).toEqual([]);
        expect(result.jobs.build.reasons.join()).toContain('Prerequisite');
    });

    it.each(['packages/webfont-generator/templates/css.hbs', 'packages/vite-svg-2-webfont/src/fixtures/webfont-test/svg/add.svg'])(
        'includes reverse engine consumers of %s',
        path => {
            const result = selectJobs([path], [packageNames.plugin]);
            expect(result.rustSuites).toEqual(['engine', 'cli', 'adapter']);
            expect(result.jobs['native-coverage'].selected).toBe(true);
            expect(result.jobs['test-host'].selected).toBe(true);
        },
    );

    it.each(['crates/webfont-generator/CHANGELOG.md', 'packages/webfont-generator/CHANGELOG.md'])('includes docs for external changelog %s', path => {
        expect(selectJobs([path], [packageNames.adapter]).jobs.docs.selected).toBe(true);
    });

    it('preserves the engine downstream chain returned by pnpm', () => {
        const result = selectJobs(
            ['crates/webfont-generator/src/lib.rs'],
            [packageNames.engine, packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root],
        );
        expect(result.full).toBe(false);
        expect(result.rustSuites).toEqual(['engine', 'cli', 'adapter']);
        expect(selected(result)).toEqual(['ci', 'rust-coverage', 'native-coverage', 'build', 'test-browser', 'test-host', 'test-docker', 'test-vite-compat']);
    });

    it.each(['Cargo.lock', 'pnpm-lock.yaml', '.github/workflows/main.yaml', 'scripts/ci/doctests.mjs', 'new-package/src/index.ts'])(
        'broadens shared/unclassified path %s even when pnpm only returns root',
        path => {
            const result = selectJobs([path], [packageNames.root]);
            expect(result.full).toBe(true);
            expect(Object.values(result.jobs).every(job => job.selected)).toBe(true);
        },
    );

    it('fails open for an unknown package or changed files with empty package selection', () => {
        expect(selectJobs(['packages/docs/a.md'], ['new-workspace']).full).toBe(true);
        expect(selectJobs(['packages/docs/a.md'], []).full).toBe(true);
        expect(selectJobs([], []).full).toBe(false);
    });

    it('broadens manifest changes, including deletion of an entire package', () => {
        expect(selectJobs(['packages/docs/package.json'], [packageNames.root]).full).toBe(true);
        expect(selectJobs(['crates/webfont-generator/Cargo.toml'], [packageNames.engine]).full).toBe(true);
    });

    it('accounts for both sides of a cross-package rename', () => {
        const result = selectJobs(['packages/docs/old.md', 'packages/example/new.md'], [packageNames.docs, packageNames.example]);
        expect(result.jobs.docs.selected).toBe(true);
        expect(result.jobs.build.selected).toBe(true);
        expect(result.jobs['test-vite-compat'].selected).toBe(true);
    });
});

describe('comparison and query failure handling', () => {
    it.each(['pull_request', 'push'])('uses a verified merge-base for %s', eventName => {
        const calls: string[][] = [];
        const result = resolveSelection({ eventName, head, base }, (command, args) => {
            calls.push([command, ...args]);
            if (args[0] === 'rev-parse') return head;
            if (args[0] === 'merge-base') return base;
            if (args[0] === 'diff') return 'packages/docs/one.md\0packages/docs/two words.md\0';
            return JSON.stringify([{ name: packageNames.docs }]);
        });
        expect(result.full).toBe(false);
        expect(result.comparison.mergeBase).toBe(base);
        expect(calls).toContainEqual(['git', 'diff', '--name-only', '--no-renames', '-z', base, head, '--']);
        expect(calls).toContainEqual(['vp', 'exec', 'pnpm', '--filter', `...[${base}]`, 'list', '--depth', '-1', '--json']);
        expect(selected(result)).toEqual(['ci', 'docs']);
    });

    it.each([undefined, '0'.repeat(40), '--invalid-ref'])('falls back for missing/invalid base %s', revision => {
        const result = resolveSelection({ eventName: 'push', head, base: revision }, () => {
            throw new Error('Commands must not run');
        });
        expect(result.full).toBe(true);
        expect(result.reasons.join()).toContain('Missing or invalid comparison revision');
    });

    it('falls back when pnpm fails or emits malformed JSON', () => {
        for (const output of ['not JSON', '{}', '[{}]']) {
            const result = resolveSelection({ eventName: 'pull_request', head, base }, (_command, args) => {
                if (args[0] === 'rev-parse') return head;
                if (args[0] === 'merge-base') return base;
                if (args[0] === 'diff') return 'packages/docs/a.md\0';
                return output;
            });
            expect(result.full).toBe(true);
            expect(result.changedFiles).toEqual(['packages/docs/a.md']);
        }
        const result = resolveSelection({ eventName: 'pull_request', head, base }, () => {
            throw new Error('Git history unavailable');
        });
        expect(result.full).toBe(true);
        expect(result.reasons.join()).toContain('Git history unavailable');
    });

    it('rejects a checkout that differs from the event head', () => {
        expect(resolveSelection({ eventName: 'pull_request', head, base }, () => base).reasons.join()).toContain('Checkout does not match');
    });

    it('falls back to full validation for a rewritten push history', () => {
        const result = resolveSelection({ eventName: 'push', head, base }, (_command, args) => (args[0] === 'rev-parse' ? head : 'c'.repeat(40)));
        expect(result.full).toBe(true);
        expect(result.reasons.join()).toContain('Push rewrote history');
    });

    it.each(['workflow_dispatch', 'schedule'])('selects full validation for %s without a comparison base', eventName => {
        expect(
            resolveSelection({ eventName, head }, () => {
                throw new Error('Not needed');
            }).full,
        ).toBe(true);
    });

    it('preserves unusual paths and validates package-list output', () => {
        expect(parseChangedFiles('with spaces\0with\nnewline\0')).toEqual(['with spaces', 'with\nnewline']);
        expect(parsePackages('[]')).toEqual([]);
        expect(() => parsePackages('No projects matched')).toThrow(SyntaxError);
    });

    it('explains selection and escapes path-derived summary text', () => {
        const result = resolveSelection({ eventName: 'push', head, base }, () => {
            throw new Error('<bad>|\nref');
        });
        const summary = selectionSummary(result);
        expect(summary).toContain('only jobs explicitly not selected may be skipped');
        expect(summary).toContain('&lt;bad&gt;&#124; ref');
        expect(summary).not.toContain('<bad>');
    });
});

const results = (selection: ReturnType<typeof selectJobs>) => ({
    'affected-selection': { result: 'success' },
    ...Object.fromEntries(Object.entries(selection.jobs).map(([name, job]) => [name, { result: job.selected ? 'success' : 'skipped' }])),
});

describe('required checks and coverage finalization', () => {
    it('accepts intended skips for docs and requires no coverage upload', () => {
        const selection = selectJobs(['packages/docs/guide.md'], [packageNames.docs]);
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: false });
    });

    it('finalizes plugin coverage only after the selected matrix succeeds', () => {
        const selection = selectJobs(['packages/vite-svg-2-webfont/src/index.ts'], [packageNames.plugin]);
        expect(verifyRequiredChecks(results(selection), selection)).toEqual({ coverage: true });
        expect(() => verifyRequiredChecks({ ...results(selection), 'test-vite-compat': { result: 'failure' } }, selection)).toThrow('test-vite-compat: failure');
    });

    it.each(['failure', 'cancelled', 'skipped', undefined])('rejects a selected job result of %s', result => {
        const selection = selectJobs([], [], 'full');
        expect(() => verifyRequiredChecks({ ...results(selection), build: { result } }, selection)).toThrow('build:');
    });

    it('rejects a failed or cancelled job even if not selected', () => {
        const selection = selectJobs([], []);
        expect(() => verifyRequiredChecks({ ...results(selection), docs: { result: 'failure' } }, selection)).toThrow('docs: failure');
        expect(() => verifyRequiredChecks({ ...results(selection), docs: { result: 'cancelled' } }, selection)).toThrow('docs: cancelled');
    });

    it('does not turn missing selection or missing jobs into successful skips', () => {
        const selection = selectJobs([], []);
        expect(() => verifyRequiredChecks({}, selection)).toThrow('Affected selection did not succeed');
        expect(() => verifyRequiredChecks(results(selection), null)).toThrow('Missing or invalid selection report');
        expect(() => verifyRequiredChecks({ ...results(selection), unexpected: { result: 'skipped' } }, selection)).toThrow('job lists differ');
    });

    it('keeps the shared rust-tests flag complete on adapter-only changes', () => {
        expect(selectJobs(['packages/webfont-generator/native/lib.rs'], [packageNames.adapter]).rustSuites).toEqual(['engine', 'cli', 'adapter']);
    });

    it('requires report checks, upload failures, and explicit finalization before carryforward', () => {
        const workflow = parse(readFileSync(new URL('../.github/workflows/main.yaml', import.meta.url), 'utf8'));
        const config = parse(readFileSync(new URL('../codecov.yml', import.meta.url), 'utf8'));
        expect(config.codecov.notify.manual_trigger).toBe(true);
        expect(config.comment.after_n_builds).toBeUndefined();
        expect(Object.values(config.flags).every((flag: unknown) => (flag as { carryforward: boolean }).carryforward)).toBe(true);
        for (const job of ['rust-coverage', 'native-coverage', 'test-vite-compat']) {
            const uploads = workflow.jobs[job].steps.filter((step: { uses?: string }) => step.uses?.startsWith('codecov/codecov-action'));
            expect(uploads.length).toBeGreaterThan(0);
            for (const step of uploads) expect(step.with.fail_ci_if_error).toBe(true);
        }
        expect(workflow.jobs['required-checks'].steps.some((step: { with?: { run_command?: string } }) => step.with?.run_command === 'send-notifications')).toBe(true);
    });
});
