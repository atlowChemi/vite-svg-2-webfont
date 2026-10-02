import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, expect, it } from 'vite-plus/test';

let directory: string;
const declarations = 'export declare function generate(): void;\n';
beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'binding-regeneration-'));
    writeFileSync(join(directory, 'package.json'), JSON.stringify({ name: '@test/generator', version: '2.3.4', napi: { binaryName: 'test-generator' } }));
    writeFileSync(join(directory, 'binding.d.ts'), declarations);
});
afterEach(() => rmSync(directory, { recursive: true, force: true }));

const run = () => spawnSync(process.execPath, [join(import.meta.dirname, 'regenerate-webfont-binding.mjs'), directory], { encoding: 'utf8' });

it('regenerates the versioned loader while preserving exports and declarations without a native binary', () => {
    // Loading this input would throw; regeneration must inspect it as text only.
    writeFileSync(join(directory, 'binding.js'), 'throw new Error("Do not load the old binding");\nexport { generate }\nexport { inspectFont }\n');
    const result = run();
    expect(result.stderr).toBe('');
    expect(result.status).toBe(0);
    const binding = readFileSync(join(directory, 'binding.js'), 'utf8');
    expect([...binding.matchAll(/^export \{ (\w+) \}$/gm)].map(match => match[1])).toEqual(['generate', 'inspectFont']);
    expect(binding).toContain('@test/generator');
    expect(binding).toContain('2.3.4');
    expect(binding).toContain('test-generator');
    expect(readFileSync(join(directory, 'binding.d.ts'), 'utf8')).toBe(declarations);
    expect(readdirSync(directory).some(name => name.endsWith('.node'))).toBe(false);
    expect(run().status).toBe(0);
    expect(readFileSync(join(directory, 'binding.js'), 'utf8')).toBe(binding);
});

it('refuses to overwrite a loader when no supported exports can be recovered', () => {
    const original = '// Missing exports: requires a full build.\n';
    writeFileSync(join(directory, 'binding.js'), original);
    const result = run();
    expect(result.status).toBe(1);
    expect(result.stderr).toContain('No NAPI exports found');
    expect(readFileSync(join(directory, 'binding.js'), 'utf8')).toBe(original);
    expect(readFileSync(join(directory, 'binding.d.ts'), 'utf8')).toBe(declarations);
});
