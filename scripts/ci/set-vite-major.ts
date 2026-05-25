import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { styleText } from 'node:util';

function fail(message: string): never {
    console.error(styleText('red', message));
    process.exit(1);
}

const args = process.argv.slice(2);
const isDryRun = args.includes('--dry-run');
const viteMajorArg = args.find(arg => !arg.startsWith('--'));

if (!viteMajorArg) {
    fail('Usage: node ./scripts/ci/set-vite-major.ts <vite-major> [--dry-run]');
}

const viteMajor = Number.parseInt(viteMajorArg, 10);
if (!Number.isInteger(viteMajor) || viteMajor < 1) {
    fail(`Invalid Vite major "${viteMajorArg}"`);
}

const viteSpecifier = `^${viteMajor}.0.0`;
const workspaceFile = join(import.meta.dirname, '..', '..', 'pnpm-workspace.yaml');
const workspaceYaml = readFileSync(workspaceFile, 'utf8');
const lines = workspaceYaml.split('\n');
let currentSection = '';
let catalogViteIndex = -1;
let overridesViteIndex = -1;

for (const [index, line] of lines.entries()) {
    const sectionMatch = /^(?<section>[a-zA-Z][\w-]*):\s*$/.exec(line);
    if (sectionMatch?.groups?.section) {
        currentSection = sectionMatch.groups.section;
        continue;
    }

    if (!/^\s+vite:\s*/.test(line)) {
        continue;
    }

    if (currentSection === 'catalog') {
        catalogViteIndex = index;
    }

    if (currentSection === 'overrides') {
        overridesViteIndex = index;
    }
}

if (catalogViteIndex === -1) {
    fail('Could not find catalog.vite in pnpm-workspace.yaml');
}

if (overridesViteIndex === -1) {
    fail('Could not find overrides.vite in pnpm-workspace.yaml');
}

lines[catalogViteIndex] = `    vite: ${viteSpecifier}`;
lines[overridesViteIndex] = `    vite: 'catalog:'`;

if (!isDryRun) {
    writeFileSync(workspaceFile, lines.join('\n'));
}

console.log(styleText('green', `Configured workspace Vite dependency to ${viteSpecifier}`));
