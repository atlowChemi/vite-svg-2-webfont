import { appendFileSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { inflateSync, brotliDecompressSync } from 'node:zlib';

function tables(font, woff) {
    const result = new Map();
    const count = font.readUInt16BE(woff ? 12 : 4);
    for (let i = 0; i < count; i++) {
        const entry = (woff ? 44 : 12) + i * (woff ? 20 : 16);
        const tag = font.toString('ascii', entry, entry + 4);
        const offset = font.readUInt32BE(entry + (woff ? 4 : 8));
        const length = font.readUInt32BE(entry + (woff ? 8 : 12));
        const payload = font.subarray(offset, offset + length);
        result.set(tag, woff && length < font.readUInt32BE(entry + 12) ? inflateSync(payload) : payload);
    }
    return result;
}

// WOFF2 stores a Brotli stream after its variable-length table directory.
// This compares the transformed stream, not reconstructed SFNT tables.
function woff2Stream(font) {
    let offset = 48;
    const skipBase128 = () => {
        for (let i = 0; i < 5; i++) {
            if (!(font.readUInt8(offset++) & 0x80)) return;
        }
        throw new Error('Invalid WOFF2 UIntBase128');
    };
    for (let i = 0; i < font.readUInt16BE(12); i++) {
        const flags = font.readUInt8(offset++);
        const index = flags & 63;
        const tag = index === 63 ? font.toString('ascii', offset, offset + 4) : null;
        if (index === 63) offset += 4;
        skipBase128();
        const glyfOrLoca = index === 10 || index === 11 || tag === 'glyf' || tag === 'loca';
        if (glyfOrLoca ? flags >> 6 === 0 : flags >> 6 !== 0) skipBase128();
    }
    return brotliDecompressSync(font.subarray(offset, offset + font.readUInt32BE(20)));
}

export function diagnose(name, left, right) {
    if (name.endsWith('.ttf') || name.endsWith('.woff')) {
        const a = tables(left, name.endsWith('.woff'));
        const b = tables(right, name.endsWith('.woff'));
        const changed = [...new Set([...a.keys(), ...b.keys()])]
            .toSorted((first, second) => first.localeCompare(second, 'en'))
            .filter(tag => !a.has(tag) || !b.has(tag) || !a.get(tag).equals(b.get(tag)));
        return changed.length ? `uncompressed tables differ: ${changed.join(', ')}` : 'table payloads identical; container/compression differs';
    }
    if (name.endsWith('.woff2')) {
        return woff2Stream(left).equals(woff2Stream(right))
            ? 'transformed stream identical; container/compression differs'
            : 'decompressed transformed stream differs; inspect companion TTF table differences';
    }
    return name.endsWith('.eot') ? 'EOT bytes differ; inspect companion TTF table differences' : 'text bytes differ';
}

function filesUnder(directory, prefix = '') {
    return readdirSync(directory, { withFileTypes: true })
        .flatMap(entry => {
            const name = `${prefix}${entry.name}`;
            return entry.isDirectory() ? filesUnder(join(directory, entry.name), `${name}/`) : [name];
        })
        .toSorted((a, b) => a.localeCompare(b, 'en'));
}

export function compareDirectories(baseline, candidate) {
    const a = new Set(filesUnder(baseline));
    const b = new Set(filesUnder(candidate));
    if (!a.size || !b.size) throw new Error('Empty comparison corpus');
    const differences = [];
    for (const name of [...new Set([...a, ...b])].toSorted((first, second) => first.localeCompare(second, 'en'))) {
        if (!a.has(name) || !b.has(name)) {
            differences.push(`${name}: missing from ${a.has(name) ? 'candidate' : 'baseline'}`);
            continue;
        }
        const left = readFileSync(join(baseline, name));
        const right = readFileSync(join(candidate, name));
        if (!left.equals(right)) differences.push(`${name}: ${diagnose(name, left, right)} (${left.length} / ${right.length} bytes)`);
    }
    return differences;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    const [root = 'artifacts/platform-output', ...requested] = process.argv.slice(2);
    const targets = requested.length
        ? requested
        : JSON.parse(readFileSync('packages/webfont-generator/package.json', 'utf8'))
              .napi.targets.toSorted((first, second) => Number(second === 'x86_64-unknown-linux-gnu') - Number(first === 'x86_64-unknown-linux-gnu'))
              .map(target => `platform-output-${target}`);
    if (targets.length < 2) throw new Error('Provide an artifact root and at least two target directories (baseline first)');
    const baseline = join(root, targets[0], 'fresh');
    const baselineEnvironment = JSON.parse(readFileSync(join(root, targets[0], 'environment.json'), 'utf8'));
    const lines = ['# Platform output parity (diagnostic)', '', `Baseline: ${targets[0]}/fresh`, ''];
    let count = 0;
    for (const target of targets) {
        // Missing artifacts are infrastructure failures, never a successful parity result.
        const environment = JSON.parse(readFileSync(join(root, target, 'environment.json'), 'utf8'));
        if (environment.revision !== baselineEnvironment.revision) throw new Error(`${target}: cannot compare different source revisions`);
        if (!requested.length && environment.build?.target !== target.slice('platform-output-'.length)) throw new Error(`${target}: missing or incorrect release provenance`);
        lines.push(`## ${target}`, '', `Node ${environment.node}; ${environment.rust}; revision ${environment.revision}`, '');
        if (environment.build) lines.push(`Release binding SHA-256: ${environment.build.sha256}`, '');
        for (const mode of ['fresh', 'incremental']) {
            const differences = compareDirectories(baseline, join(root, target, mode));
            count += differences.length;
            lines.push(`### ${mode}: ${differences.length} differing files`, '', ...differences.map(line => `- ${line}`), '');
        }
    }
    lines.push(count ? `Diagnostic only: ${count} file comparisons differ.` : 'All compared files are byte-identical.');
    const report = `${lines.join('\n')}\n`;
    console.log(report);
    if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, report);
    if (count && process.env.GITHUB_ACTIONS) console.log(`::warning::Platform output parity: ${count} differing file comparisons (diagnostic only)`);
}
