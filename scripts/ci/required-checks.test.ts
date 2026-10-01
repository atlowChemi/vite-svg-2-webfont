import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, expect, it } from 'vite-plus/test';
import { packageNames, resolveSelection, selectJobs, type Selection } from './affected-selection';

const script = fileURLToPath(new URL('./required-checks.ts', import.meta.url));
let directory: string;
let output: string;

beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'required-checks-'));
    output = join(directory, 'output');
    writeFileSync(output, '');
});

afterEach(() => rmSync(directory, { recursive: true, force: true }));

function needs(selection: Selection) {
    return {
        ...Object.fromEntries(Object.entries(selection.jobs).map(([name, selected]) => [name, { result: selected ? 'success' : 'skipped' }])),
        'affected-selection': { result: 'success', outputs: { selection: JSON.stringify(selection) } },
    };
}

function run(input: string) {
    return spawnSync(process.execPath, [script], {
        cwd: directory,
        encoding: 'utf8',
        env: { ...process.env, NEEDS_JSON: input, GITHUB_OUTPUT: output },
    });
}

it('emits one GitHub error per failed job without a stack trace or coverage output', () => {
    const input = { ...needs(resolveSelection(undefined)), ci: { result: 'failure' }, docs: { result: 'cancelled' } };
    const result = run(JSON.stringify(input));
    expect(result.status).toBe(1);
    expect(result.stderr.trim().split('\n')).toEqual([
        '::error title=Required checks failed::ci: failure (selected=true)',
        '::error title=Required checks failed::docs: cancelled (selected=true)',
    ]);
    expect(result.stdout).toBe('');
    expect(readFileSync(output, 'utf8')).toBe('');
});

it.each(['{invalid', JSON.stringify({ 'affected-selection': { result: 'success', outputs: { selection: '{invalid' } } })])(
    'reports malformed input as an annotation: %s',
    input => {
        const result = run(input);
        expect(result.status).toBe(1);
        expect(result.stderr).toMatch(/^::error title=Required checks failed::/);
        expect(result.stderr).not.toMatch(/\n\s+at |Node\.js v/);
        expect(readFileSync(output, 'utf8')).toBe('');
    },
);

it('escapes workflow-command data', () => {
    const input = { ...needs(resolveSelection(undefined)), ci: { result: 'failure%0A::warning::injected\r' } };
    const result = run(JSON.stringify(input));
    expect(result.status).toBe(1);
    expect(result.stderr).toBe('::error title=Required checks failed::ci: failure%250A::warning::injected%0D (selected=true)\n');
});

it.each([true, false])('preserves successful finalization output: coverage=%s', coverage => {
    const selection = resolveSelection(undefined);
    if (!coverage) {
        selection.packages = [packageNames.docs];
        selection.jobs = selectJobs(selection.packages, ['packages/docs/guide.md']);
        selection.rustSuites = [];
        selection.nativeBuildScope = 'linux-x64';
    }
    const result = run(JSON.stringify(needs(selection)));
    expect(result.status).toBe(0);
    expect(result.stderr).toBe('');
    expect(result.stdout.trim()).toBe('::notice title=Required checks succeeded::All selected jobs succeeded; all skips were explicitly permitted.');
    expect(readFileSync(output, 'utf8')).toBe(`coverage=${coverage}\n`);
});
