import { globSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';
import { expect, it } from 'vite-plus/test';
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
        },
        scripts,
        projects,
        auditTasks,
    );
    expect(work.workers.flatMap(worker => worker.pipelines)).toEqual(expect.arrayContaining(['next-plugin-test', 'next-docs', 'next-example', 'next-new-validation']));
});

it('creates no workers for an inert change and falls back to all work without a base', () => {
    expect(selectAuditWork(select(['packages/docs/wrangler.jsonc'], []), pipelines, scripts, projects, auditTasks).workers).toEqual([]);
    expect(
        selectAuditWork(resolveSelection(undefined), pipelines, scripts, projects, auditTasks)
            .workers.flatMap(worker => worker.audits)
            .toSorted(),
    ).toEqual(auditTasks.toSorted());
});

it('emits a usable CI matrix from the package selection and discovered policy tasks', () => {
    const directory = mkdtempSync(join(tmpdir(), 'audit-selection-'));
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
        write('crates/cache-audit/policy.yaml', "version: 1\ntasks:\n  '.#next:docs': {}\n  '.#next:native': {}\n");
        execFileSync(process.execPath, [fileURLToPath(new URL('./select-audit-work.ts', import.meta.url))], {
            cwd: directory,
            env: { ...process.env, GITHUB_OUTPUT: join(directory, 'output'), GITHUB_STEP_SUMMARY: join(directory, 'summary') },
        });
        const result = JSON.parse(readFileSync(join(directory, 'output'), 'utf8').trim().slice('work='.length));
        expect(result.workers).toEqual([{ id: 0, pipelines: ['next-docs'], audits: ['next:docs'] }]);
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});
