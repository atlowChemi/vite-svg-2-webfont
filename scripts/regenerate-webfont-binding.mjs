import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { readNapiConfig, writeJsBinding } from '@napi-rs/cli';

// Release preparation changes versions, not the native API. Preserve the exports from
// the last full NAPI build without loading its platform-specific native binary.
// A normal binding build remains responsible for API changes and binding.d.ts.
const packageDir = process.argv[2] ?? fileURLToPath(new URL('../packages/webfont-generator/', import.meta.url));
const binding = await readFile(join(packageDir, 'binding.js'), 'utf8');
const idents = [...binding.matchAll(/^export \{ (\w+) \}$/gm)].map(match => match[1]);
if (idents.length === 0) {
    throw new Error('No NAPI exports found in binding.js; run a full binding build before release preparation.');
}
const config = await readNapiConfig(join(packageDir, 'package.json'));
await writeJsBinding({
    platform: true,
    esm: true,
    jsBinding: 'binding.js',
    idents,
    binaryName: config.binaryName,
    packageName: config.packageName,
    version: config.packageJson.version,
    outputDir: packageDir,
});
