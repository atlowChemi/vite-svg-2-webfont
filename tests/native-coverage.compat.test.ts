import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterEach, expect, test } from 'vite-plus/test';

const dirs: string[] = [];
afterEach(() => {
    for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true });
});
const root = resolve(import.meta.dirname, '..');
const record = (path: string, hits: number) => `SF:${path}\nDA:1,${hits}\nLF:1\nLH:${hits ? 1 : 0}\nend_of_record\n`;
function validate(text: string, mode = 'native') {
    const dir = mkdtempSync(join(tmpdir(), 'webfont-coverage-'));
    dirs.push(dir);
    const file = join(dir, 'rust.lcov');
    writeFileSync(file, text);
    const result = spawnSync(process.execPath, [resolve(root, 'scripts/ci/validate-native-coverage.mjs'), file, mode], { encoding: 'utf8' });
    return { ...result, text: readFileSync(file, 'utf8') };
}

test('requires measured engine and adapter execution, not just an existing LCOV file', () => {
    expect(validate('').status).not.toBe(0);
    const engine = record('crates/webfont-generator/src/lib.rs', 1);
    expect(validate(engine).status).not.toBe(0);
    expect(validate(engine + record('packages/webfont-generator/native/lib.rs', 0)).status).not.toBe(0);
    expect(validate(engine + record('packages/webfont-generator/native/lib.rs', 1)).status).toBe(0);
});

test('normalizes native sources and package-local JS sources to repository paths', () => {
    const result = validate(record(`${root}/crates/webfont-generator/src/lib.rs`, 1) + record(`${root}/packages/webfont-generator/native/lib.rs`, 1));
    expect(result.status).toBe(0);
    expect(result.text).not.toContain(root);
    const js = validate(record('index.js', 1), 'js');
    expect(js.status).toBe(0);
    expect(js.text).toContain('SF:packages/webfont-generator/index.js');
});
