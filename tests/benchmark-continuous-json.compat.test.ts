import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, expect, test } from 'vite-plus/test';

const root = fileURLToPath(new URL('../', import.meta.url));
const script = fileURLToPath(new URL('../scripts/benchmark-continuous-json.mjs', import.meta.url));
let directory: string;

beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'benchmark-report-'));
    const criterion = join(directory, 'criterion', 'pipeline', 'new');
    mkdirSync(criterion, { recursive: true });
    writeFileSync(
        join(criterion, 'estimates.json'),
        JSON.stringify({ mean: { point_estimate: 2_000_000, confidence_interval: { lower_bound: 1_000_000, upper_bound: 3_000_000 } } }),
    );
});

afterEach(() => {
    rmSync(directory, { recursive: true, force: true });
});

function convert(report: unknown) {
    const input = join(directory, 'vitest.json');
    const output = join(directory, 'output', 'continuous.json');
    writeFileSync(input, JSON.stringify(report));
    const processResult = spawnSync(process.execPath, [script, output, join(directory, 'criterion'), input], { cwd: root, encoding: 'utf8' });
    return { ...processResult, output };
}

test('converts v5 comparisons and standalone results while preserving historical names and units', () => {
    const result = convert({
        testResults: [
            {
                name: join(root, 'tests/webfonts-generator.bench.ts'),
                assertionResults: [
                    {
                        ancestorTitles: ['100 glyphs', 'optimize SVG: true'],
                        title: 'all formats',
                        benchmarks: [
                            {
                                name: 'comparison',
                                tasks: [
                                    { name: 'upstream', latency: { mean: 12, rme: 2 }, throughput: { mean: 83 } },
                                    { name: 'new core', latency: { mean: 3, rme: 1 }, throughput: { mean: 333 } },
                                ],
                            },
                        ],
                    },
                    {
                        ancestorTitles: ['dependency-aware render reuse add/remove toggle — 100 glyphs'],
                        title: 'new core — default CSS rerender toggle',
                        benchmarks: [{ name: 'standalone', tasks: [{ name: 'new core — default CSS rerender toggle', latency: { mean: 0.25 }, throughput: { mean: 4000 } }] }],
                    },
                    { ancestorTitles: [], title: 'skipped', benchmarks: [] },
                ],
            },
        ],
    });
    expect(result.stderr).toBe('');
    expect(result.status).toBe(0);
    expect(JSON.parse(readFileSync(result.output, 'utf8'))).toEqual([
        { name: 'criterion/pipeline', unit: 'ms', value: 2, range: '1..3' },
        { name: 'vitest/tests/webfonts-generator.bench.ts > 100 glyphs > optimize SVG: true > all formats > upstream', unit: 'ms', value: 12, range: '± 2%', extra: '83 ops/sec' },
        { name: 'vitest/tests/webfonts-generator.bench.ts > 100 glyphs > optimize SVG: true > all formats > new core', unit: 'ms', value: 3, range: '± 1%', extra: '333 ops/sec' },
        {
            name: 'vitest/tests/webfonts-generator.bench.ts > dependency-aware render reuse add/remove toggle — 100 glyphs > new core — default CSS rerender toggle',
            unit: 'ms',
            value: 0.25,
            extra: '4000 ops/sec',
        },
    ]);
});

test.each([{ testResults: [] }, { files: [{ groups: [{ benchmarks: [{ name: 'v4 result', mean: 1 }] }] }] }])('rejects reports without v5 benchmark measurements: %j', report => {
    const result = convert(report);
    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain('No Vitest benchmark measurements found');
});

test('rejects missing latency rather than silently dropping a benchmark', () => {
    const result = convert({
        testResults: [{ assertionResults: [{ benchmarks: [{ tasks: [{ name: 'broken', latency: {} }] }] }] }],
    });
    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain('Missing latency mean for Vitest benchmark: broken');
});
