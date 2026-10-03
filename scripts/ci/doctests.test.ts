import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
// @ts-expect-error Standalone JavaScript CLI has no declaration file.
import { runDoctests } from './doctests.mjs';

vi.mock('node:child_process', () => ({ spawnSync: vi.fn() }));
let directory: string;
const result = (status: number | null, stdout = '', stderr = '') => ({ pid: 1, status, signal: null, output: [null, stdout, stderr], stdout, stderr });
beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'doctest-report-'));
    vi.spyOn(process.stdout, 'write').mockReturnValue(true);
    vi.spyOn(process.stderr, 'write').mockReturnValue(true);
});
afterEach(() => {
    vi.restoreAllMocks();
    vi.mocked(spawnSync).mockReset();
    rmSync(directory, { recursive: true, force: true });
});
const report = () => readFileSync(join(directory, 'test-results/doctests.xml'), 'utf8');

it('executes both feature modes and retains escaped output in a successful report', () => {
    vi.mocked(spawnSync).mockReturnValue(result(0, 'ok <doc> & "example"', 'warning <&>'));
    expect(runDoctests(directory)).toBe(0);
    expect(spawnSync).toHaveBeenNthCalledWith(1, 'cargo', ['test', '--locked', '-p', 'webfont-generator', '--doc'], expect.objectContaining({ cwd: directory }));
    expect(spawnSync).toHaveBeenNthCalledWith(
        2,
        'cargo',
        ['test', '--locked', '-p', 'webfont-generator', '--doc', '--features', 'cli'],
        expect.objectContaining({ cwd: directory }),
    );
    expect(report()).toContain('tests="2" failures="0"');
    expect(report()).toContain('name="default (aggregate)"');
    expect(report()).toContain('name="cli (aggregate)"');
    expect(report()).toContain('ok &lt;doc&gt; &amp; &quot;example&quot;');
    expect(report()).toContain('<system-err>warning &lt;&amp;&gt;</system-err>');
    expect(report()).not.toContain('<failure');
});

it.each([0, 1])('fails the aggregate when invocation %i fails and writes results before the next invocation', failing => {
    vi.mocked(spawnSync)
        .mockReturnValueOnce(result(failing === 0 ? 1 : 0, 'first output', failing === 0 ? 'bad <&>' : ''))
        .mockImplementationOnce(() => {
            expect(report()).toContain('tests="1"');
            expect(report()).toContain('first output');
            return result(failing === 1 ? 1 : 0, 'second output', failing === 1 ? 'bad <&>' : '');
        });
    expect(runDoctests(directory)).toBe(1);
    expect(report()).toContain('tests="2" failures="1"');
    expect(report()).toContain('<failure message="Cargo doctests failed">bad &lt;&amp;&gt;</failure>');
    expect(report()).toContain('first output');
    expect(report()).toContain('second output');
});

it('reports spawn errors as failures even when no process output exists', () => {
    // Node returns null streams on spawn failure despite its declared stream types.
    vi.mocked(spawnSync).mockReturnValue({ ...result(null), stdout: null, stderr: null, error: new Error('spawn cargo ENOENT') } as unknown as ReturnType<typeof spawnSync>);
    expect(runDoctests(directory)).toBe(1);
    expect(report()).toContain('tests="2" failures="2"');
    expect(report()).toContain('<failure message="Cargo doctests failed">spawn cargo ENOENT</failure>');
});
