import { globSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';
import { expect, it, vi } from 'vite-plus/test';
import { packageNames, resolveSelection } from './affected-selection';
import { selectAuditWork } from './select-audit-work';

const { pipelines } = parse(readFileSync('pnpm-tasks.yaml', 'utf8'));
const { scripts } = JSON.parse(readFileSync('package.json', 'utf8'));
const projects = Object.fromEntries(
    globSync(['crates/*/package.json', 'packages/*/package.json']).map(file => [file.replace(/\/package.json$/, ''), JSON.parse(readFileSync(file, 'utf8')).name]),
);
const auditTasks = ['next:native', 'next:wasm', 'next:docs', 'next:plugin:pack'];
const select = (files: string[], packages: string[]) =>
    resolveSelection('a'.repeat(40), (_command, args) => {
        if (args[0] === 'merge-base') return 'a'.repeat(40);
        if (args[0] === 'diff') return files.join('\0');
        return JSON.stringify(packages.map(name => ({ name })));
    });

it('limits docs-only changes to docs pipelines and audits', () => {
    const work = selectAuditWork(select(['packages/docs/index.md'], [packageNames.docs]), pipelines, scripts, projects, auditTasks);
    const selected = work.workers.flatMap(worker => worker.pipelines);
    expect(selected).toContain('next-docs');
    expect(selected).not.toContain('next-native');
    expect(selected).not.toContain('next-test');
    expect(selected).not.toContain('next-browser-test');
    expect(work.workers.flatMap(worker => worker.audits)).toEqual(['next:docs']);
});

it('runs every discovered pipeline exactly once in at most two workers for shared changes', () => {
    const discovered = { ...pipelines, 'next-new-validation': ['next:new'] };
    const work = selectAuditWork(select(['Cargo.lock'], []), discovered, scripts, projects, auditTasks);
    expect(work.workers).toHaveLength(2);
    expect(work.workers.flatMap(worker => worker.pipelines).toSorted()).toEqual(Object.keys(discovered).toSorted());
    expect(work.workers.flatMap(worker => worker.audits).toSorted()).toEqual(auditTasks.toSorted());
});

it('keeps downstream packages and conservatively includes unknown commands', () => {
    const work = selectAuditWork(
        select(['packages/vite-svg-2-webfont/src/index.ts'], [packageNames.plugin, packageNames.docs, packageNames.example]),
        {
            ...pipelines,
            'next-new-validation': ['next:new'],
            'next-unknown-project': ['next:unknown'],
        },
        { ...scripts, 'next:unknown': 'pnpm --dir packages/unknown run next:check' },
        projects,
        auditTasks,
    );
    expect(work.workers.flatMap(worker => worker.pipelines)).toEqual(
        expect.arrayContaining(['next-plugin-test', 'next-docs', 'next-example', 'next-new-validation', 'next-unknown-project']),
    );
});

it('creates no workers for an inert change and falls back to all work without a base', () => {
    expect(selectAuditWork(select(['packages/docs/wrangler.jsonc'], []), pipelines, scripts, projects, auditTasks).workers).toEqual([]);
    expect(
        selectAuditWork(resolveSelection(undefined), pipelines, scripts, projects, auditTasks)
            .workers.flatMap(worker => worker.audits)
            .toSorted(),
    ).toEqual(auditTasks.toSorted());
});

/* oxlint-disable vitest/no-conditional-expect -- Each policy fixture has a distinct expected outcome. */
it.each(['policy', 'no policy', 'invalid policy'] as const)('runs the CI entry point with %s', async policy => {
    const directory = mkdtempSync(join(tmpdir(), 'audit-selection-'));
    const cwd = process.cwd();
    const argv = process.argv;
    const write = (file: string, value: unknown) => {
        const path = join(directory, file);
        mkdirSync(dirname(path), { recursive: true });
        writeFileSync(path, typeof value === 'string' ? value : JSON.stringify(value));
    };
    try {
        write('artifacts/affected-selection/selection.json', select(['packages/docs/index.md'], [packageNames.docs]));
        write('pipelines.json', { 'next-docs': ['next:docs'], 'next-native': ['next:native'], 'next-docs-dev': ['next:docs'] });
        write('selected-pipelines.txt', 'next-docs\nnext-native\n');
        write('package.json', { scripts });
        write('packages/docs/package.json', { name: packageNames.docs });
        write('packages/webfont-generator/package.json', { name: packageNames.adapter });
        if (policy !== 'no policy') {
            write('crates/cache-audit/policy.yaml', `version: 1\ntasks:\n  '${policy === 'invalid policy' ? 'other#docs' : '.#next:docs'}': {}\n  '.#next:native': {}\n`);
        }
        // Run the entry point through Vitest too: native Node TS and Vite transforms have different coverage offsets.
        process.chdir(directory);
        process.argv = [process.execPath, fileURLToPath(new URL('./select-audit-work.ts', import.meta.url))];
        vi.stubEnv('GITHUB_OUTPUT', policy === 'no policy' ? undefined : join(directory, 'output'));
        vi.stubEnv('GITHUB_STEP_SUMMARY', policy === 'no policy' ? undefined : join(directory, 'summary'));
        const log = vi.spyOn(console, 'log').mockImplementation(() => {});
        vi.resetModules();
        if (policy === 'invalid policy') {
            await expect(import('./select-audit-work')).rejects.toThrow('Unsupported audit project: other#docs');
            return;
        }
        await import('./select-audit-work');
        const result = JSON.parse(log.mock.calls[0]![0]);
        expect(result.workers).toEqual([{ id: 0, pipelines: ['next-docs'], audits: policy === 'no policy' ? [] : ['next:docs'] }]);
        if (policy === 'policy') {
            expect(readFileSync(join(directory, 'output'), 'utf8')).toBe(`work=${JSON.stringify(result)}\n`);
            expect(readFileSync(join(directory, 'summary'), 'utf8')).toContain(JSON.stringify(result, null, 2));
        }
        process.argv = [];
        vi.resetModules();
        await import('./select-audit-work');
        expect(log).toHaveBeenCalledOnce();
    } finally {
        process.chdir(cwd);
        process.argv = argv;
        vi.unstubAllEnvs();
        vi.restoreAllMocks();
        rmSync(directory, { recursive: true, force: true });
    }
});
/* oxlint-enable vitest/no-conditional-expect */
