import process from 'node:process';
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
    if (['test-host', 'test-docker', 'test-vite-compat', 'platform-output'].some(name => decisions[name]) && !decisions.build) {
        failures.push('Selected artifact consumer lacks a selected native build');
    }
    if (!['full', 'linux-x64'].includes(selection.nativeBuildScope)) failures.push('Missing or invalid native build scope');
    if ((decisions['test-host'] || decisions['test-docker'] || decisions['platform-output']) && selection.nativeBuildScope !== 'full') {
        failures.push('Platform tests require the full native build scope');
    }
    const suites = selection.packages?.includes('@atlowchemi/webfont-engine')
        ? ['engine', 'cli', 'adapter']
        : selection.packages?.includes('@atlowchemi/webfont-generator')
          ? ['adapter']
          : [];
    if (JSON.stringify(selection.rustSuites) !== JSON.stringify(suites) || decisions['rust-coverage'] !== suites.length > 0) {
        failures.push('Rust coverage suites do not match affected packages');
    }
    if (failures.length) throw new Error(failures.join('\n'));
    return { coverage: ['test-scripts', 'rust-coverage', 'native-coverage', 'test-vite-compat'].some(name => decisions[name]) };
}

const { argv, env } = process;

const executedFile = argv[1];
const currentModuleAbsoluteUrl = import.meta.url;
const { href: executedFileHref } = (executedFile && pathToFileURL(executedFile)) || { href: '' };

if (executedFile && currentModuleAbsoluteUrl === executedFileHref) {
    try {
        const needs = JSON.parse(env.NEEDS_JSON ?? '{}');
        const selection = JSON.parse(needs['affected-selection']?.outputs?.selection ?? 'null');
        const result = verifyRequiredChecks(needs, selection);

        if (env.GITHUB_OUTPUT) {
            appendFileSync(env.GITHUB_OUTPUT, `coverage=${result.coverage}\n`);
        }
        console.log('::notice title=Required checks succeeded::All selected jobs succeeded; all skips were explicitly permitted.');
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        for (const failure of message.split('\n')) {
            // Escape workflow-command data so messages remain literal annotation text.
            console.error(`::error title=Required checks failed::${failure.replaceAll('%', '%25').replaceAll('\r', '%0D')}`);
        }
        process.exitCode = 1;
    }
}
