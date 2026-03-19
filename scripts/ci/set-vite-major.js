import { readFileSync, writeFileSync } from 'node:fs';

const args = process.argv.slice(2);
const isDryRun = args.includes('--dry-run');
const viteMajorArg = args.find(arg => !arg.startsWith('--'));

if (!viteMajorArg) {
    console.error('Usage: node ./scripts/ci/set-vite-major.js <vite-major> [--dry-run]');
    process.exit(1);
}

const viteMajor = Number.parseInt(viteMajorArg, 10);
if (!Number.isInteger(viteMajor) || viteMajor < 1) {
    console.error(`Invalid Vite major "${viteMajorArg}"`);
    process.exit(1);
}

const viteSpecifier = `^${viteMajor}.0.0`;
const workspaceFile = new URL('../../pnpm-workspace.yaml', import.meta.url);
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

    if (!line.includes('vite:')) {
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
    console.error('Could not find catalog.vite in pnpm-workspace.yaml');
    process.exit(1);
}

if (overridesViteIndex === -1) {
    console.error('Could not find overrides.vite in pnpm-workspace.yaml');
    process.exit(1);
}

lines[catalogViteIndex] = `    vite: ${viteSpecifier}`;
lines[overridesViteIndex] = `    vite: 'catalog:'`;

const nextWorkspaceYaml = lines.join('\n');

if (!isDryRun) {
    writeFileSync(workspaceFile, nextWorkspaceYaml);
}

console.log(`Configured workspace Vite dependency to ${viteSpecifier}`);
