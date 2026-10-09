import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { brotliCompressSync, constants, deflateSync } from 'node:zlib';
import { afterEach, expect, it } from 'vite-plus/test';
// @ts-expect-error Standalone JavaScript CLI has no declaration file.
import { compareDirectories, diagnose } from './compare-platform-output.mjs';

const roots: string[] = [];
afterEach(() => {
    for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

it('compares exact bytes and reports missing/extra files instead of comparing only the intersection', () => {
    const root = mkdtempSync(join(tmpdir(), 'platform-output-'));
    roots.push(root);
    const a = join(root, 'a');
    const b = join(root, 'b');
    for (const path of [a, b]) {
        mkdirSync(path);
        writeFileSync(join(path, 'font.svg'), '<svg/>');
    }
    expect(compareDirectories(a, b)).toEqual([]);
    writeFileSync(join(b, 'font.svg'), '<svg />');
    writeFileSync(join(a, 'font.css'), 'css');
    writeFileSync(join(b, 'font.html'), 'html');
    expect(compareDirectories(a, b)).toEqual(['font.css: missing from candidate', 'font.html: missing from baseline', 'font.svg: text bytes differ (6 / 7 bytes)']);
    expect(() => compareDirectories(a, join(root, 'missing'))).toThrow(/ENOENT/);
    const empty = join(root, 'empty');
    mkdirSync(empty);
    expect(() => compareDirectories(a, empty)).toThrow('Empty comparison corpus');
});

function sfnt(payload: Buffer, woff = false, level = 6): Buffer {
    const encoded = woff ? deflateSync(payload, { level }) : payload;
    const offset = woff ? 64 : 28;
    const font = Buffer.alloc(offset);
    font.writeUInt16BE(1, woff ? 12 : 4);
    font.write('glyf', woff ? 44 : 12);
    font.writeUInt32BE(offset, woff ? 48 : 20);
    font.writeUInt32BE(encoded.length, woff ? 52 : 24);
    if (woff) font.writeUInt32BE(payload.length, 56);
    return Buffer.concat([font, encoded]);
}

it('distinguishes changed TTF/WOFF table bytes from different compression', () => {
    const payload = Buffer.from('abc'.repeat(100));
    const changed = Buffer.from('xyz'.repeat(100));
    expect(diagnose('font.ttf', sfnt(payload), sfnt(changed))).toBe('uncompressed tables differ: glyf');
    expect(diagnose('font.woff', sfnt(payload, true, 1), sfnt(payload, true, 9))).toBe('table payloads identical; container/compression differs');
    expect(diagnose('font.woff', sfnt(payload, true), sfnt(changed, true))).toBe('uncompressed tables differ: glyf');
});

function woff2(payload: Buffer, quality: number): Buffer {
    const compressed = brotliCompressSync(payload, { params: { [constants.BROTLI_PARAM_QUALITY]: quality } });
    const header = Buffer.alloc(48);
    header.writeUInt16BE(3, 12);
    header.writeUInt32BE(compressed.length, 20);
    // Transformed glyf/loca, then a custom untransformed tag; multi-byte original length.
    const directory = Buffer.from([10, 0x82, 0x2c, 3, 11, 4, 0, 63, ...Buffer.from('TEST'), 1]);
    return Buffer.concat([header, directory, compressed]);
}

it('compares decompressed WOFF2 transformed streams across different Brotli settings', () => {
    const payload = Buffer.from('abc'.repeat(100));
    expect(diagnose('font.woff2', woff2(payload, 1), woff2(payload, 11))).toBe('transformed stream identical; container/compression differs');
    expect(diagnose('font.woff2', woff2(payload, 1), woff2(Buffer.from('changed'), 1))).toContain('decompressed transformed stream differs');
    expect(() => diagnose('font.woff2', Buffer.alloc(1), woff2(payload, 1))).toThrow(/bounds|range/i);
});

it('keeps byte mismatches diagnostic but fails when an expected platform artifact is missing', () => {
    const root = mkdtempSync(join(tmpdir(), 'platform-output-cli-'));
    roots.push(root);
    for (const target of ['linux-x64', 'darwin-arm64']) {
        for (const mode of ['fresh', 'incremental']) {
            const path = join(root, target, mode);
            mkdirSync(path, { recursive: true });
            writeFileSync(join(path, 'font.css'), target);
        }
        writeFileSync(join(root, target, 'environment.json'), JSON.stringify({ node: 'test', rust: 'test', revision: 'test' }));
    }
    const run = (targets: string[]) =>
        spawnSync(process.execPath, ['scripts/ci/compare-platform-output.mjs', root, ...targets], {
            encoding: 'utf8',
            env: { ...process.env, GITHUB_STEP_SUMMARY: join(root, 'summary.md'), GITHUB_ACTIONS: 'true' },
        });
    const diagnostic = run(['linux-x64', 'darwin-arm64']);
    expect(diagnostic.status).toBe(0);
    expect(diagnostic.stdout).toContain('::warning::Platform output parity: 2 differing file comparisons');
    expect(run(['linux-x64', 'win32-x64']).status).not.toBe(0);
});
