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

// Vite+ requires its branded vite alias. Install the consumer's Vite separately;
// the plugin test project redirects its Vite imports when VITE_COMPAT_MAJOR is set.
const viteSpecifier = `npm:vite@^${viteMajor}.0.0`;
const packageFile = join(import.meta.dirname, '..', '..', 'package.json');
const packageJson = JSON.parse(readFileSync(packageFile, 'utf8'));
packageJson.devDependencies ??= {};
packageJson.devDependencies['vite-compat'] = viteSpecifier;

if (!isDryRun) {
    writeFileSync(packageFile, `${JSON.stringify(packageJson, null, 4)}\n`);
}

console.log(styleText('green', `Configured vite-compat dependency to ${viteSpecifier}`));
