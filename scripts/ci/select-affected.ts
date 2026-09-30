import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolveSelection, selectionSummary } from './affected-selection.ts';

let event;
try {
    event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH ?? '', 'utf8'));
} catch (error) {
    // Missing event data leaves the comparison unavailable and selects full CI.
    console.warn('Unable to read GitHub event; defaulting to full validation.', error);
}
const selection = resolveSelection(
    {
        eventName: process.env.GITHUB_EVENT_NAME ?? 'manual',
        head: event?.pull_request?.head?.sha ?? process.env.GITHUB_SHA ?? '',
        base: event?.pull_request?.base?.sha ?? event?.before,
        // Keep a complete carryforward baseline on main; PRs use affected selection.
        full: process.env.GITHUB_EVENT_NAME === 'push' || event?.inputs?.full === true || event?.inputs?.full === 'true',
    },
    (command, args) => execFileSync(command, args, { encoding: 'utf8', timeout: 60_000, maxBuffer: 8 * 1024 * 1024 }),
);
mkdirSync('artifacts/affected-selection', { recursive: true });
writeFileSync('artifacts/affected-selection/selection.json', `${JSON.stringify(selection, null, 2)}\n`);
const summary = selectionSummary(selection);
writeFileSync('artifacts/affected-selection/summary.md', summary);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, `selection=${JSON.stringify(selection)}\n`);
}
console.log(summary);
