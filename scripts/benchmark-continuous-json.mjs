import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, sep } from 'node:path';

const [, , outputPath, criterionRoot, vitestPath] = process.argv;

if (!outputPath || !criterionRoot || !vitestPath) {
    throw new Error('Usage: node scripts/benchmark-continuous-json.mjs <output> [criterion-root] [vitest-json]');
}

const results = [];

function json(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}

function* walk(path) {
    for (const entry of readdirSync(path, { withFileTypes: true })) {
        const child = join(path, entry.name);
        if (entry.isDirectory()) {
            yield* walk(child);
        } else {
            yield child;
        }
    }
}

function nsToMs(value) {
    return value / 1_000_000;
}

for (const file of walk(criterionRoot)) {
    if (!file.endsWith(`${sep}new${sep}estimates.json`)) continue;

    const estimates = json(file);
    const mean = estimates.mean?.point_estimate;
    if (typeof mean !== 'number') continue;

    const name = relative(criterionRoot, dirname(dirname(file)))
        .split(sep)
        .join('/');
    const lower = estimates.mean?.confidence_interval?.lower_bound;
    const upper = estimates.mean?.confidence_interval?.upper_bound;

    results.push({
        name: `criterion/${name}`,
        unit: 'ms',
        value: nsToMs(mean),
        range: typeof lower === 'number' && typeof upper === 'number' ? `${nsToMs(lower)}..${nsToMs(upper)}` : undefined,
    });
}

const report = json(vitestPath);
let vitestCount = 0;

for (const file of report.testResults ?? []) {
    for (const test of file.assertionResults ?? []) {
        for (const group of test.benchmarks ?? []) {
            for (const bench of group.tasks ?? []) {
                if (!Number.isFinite(bench.latency?.mean)) {
                    throw new Error(`Missing latency mean for Vitest benchmark: ${bench.name}`);
                }

                // Preserve v4's repository-relative filename and >-joined suite names.
                // A standalone v5 test repeats its benchmark name; don't add it twice.
                const titles = test.title === bench.name ? test.ancestorTitles : [...test.ancestorTitles, test.title];
                const fileName = relative(process.cwd(), file.name).split(sep).join('/');
                const fullName = [fileName, ...titles].join(' > ');
                results.push({
                    name: `vitest/${fullName} > ${bench.name}`,
                    unit: 'ms',
                    value: bench.latency.mean,
                    range: typeof bench.latency.rme === 'number' ? `± ${bench.latency.rme}%` : undefined,
                    extra: typeof bench.throughput?.mean === 'number' ? `${bench.throughput.mean} ops/sec` : undefined,
                });
                vitestCount++;
            }
        }
    }
}

if (vitestCount === 0) {
    throw new Error('No Vitest benchmark measurements found; expected a Vitest v5 JSON reporter output.');
}

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(results, null, 2)}\n`);
