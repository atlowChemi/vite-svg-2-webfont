import { appendFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import type { Selection } from './affected-selection';

// Treat missing/malformed decisions as failures, not permission to skip checks.
export function verifyRequiredChecks(needs: Record<string, { result?: string }>, selection: Selection | null): { coverage: boolean } {
    if (needs?.['affected-selection']?.result !== 'success') throw new Error('Affected selection did not succeed');
    if (!selection?.jobs) throw new Error('Missing or invalid selection report');
    const jobs = Object.keys(needs)
        .filter(name => name !== 'affected-selection')
        .toSorted();
    if (JSON.stringify(jobs) !== JSON.stringify(Object.keys(selection.jobs).toSorted())) throw new Error('Selection and required job lists differ');
    const failures = [];
    const decisions = selection.jobs;
    for (const name of jobs) {
        const decision = decisions[name];
        const result = needs[name]?.result;
        if (typeof decision !== 'boolean') failures.push(`${name}: missing selection decision`);
        else if (result !== 'success' && !(!decision && result === 'skipped')) failures.push(`${name}: ${result ?? 'missing'} (selected=${decision})`);
    }
    if (!decisions.ci) failures.push('Shared checks must always be selected');
    if (['test-host', 'test-docker', 'test-vite-compat'].some(name => decisions[name]) && !decisions.build) {
        failures.push('Selected artifact consumer lacks a selected native build');
    }
    if (failures.length) throw new Error(failures.join('\n'));
    return { coverage: ['test-scripts', 'rust-coverage', 'native-coverage', 'test-vite-compat'].some(name => decisions[name]) };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    const needs = JSON.parse(process.env.NEEDS_JSON ?? '{}');
    const selection = JSON.parse(needs['affected-selection']?.outputs?.selection ?? 'null');
    const result = verifyRequiredChecks(needs, selection);
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `coverage=${result.coverage}\n`);
    console.log('All selected jobs succeeded; all skips were explicitly permitted.');
}
