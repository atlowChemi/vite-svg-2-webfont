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

for (const file of report.files ?? []) {
    for (const group of file.groups ?? []) {
        for (const bench of group.benchmarks ?? []) {
            if (typeof bench.mean !== 'number') continue;

            results.push({
                name: `vitest/${group.fullName} > ${bench.name}`,
                unit: 'ms',
                value: bench.mean,
                range: typeof bench.rme === 'number' ? `± ${bench.rme}%` : undefined,
                extra: typeof bench.hz === 'number' ? `${bench.hz} ops/sec` : undefined,
            });
        }
    }
}

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(results, null, 2)}\n`);
