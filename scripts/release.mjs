import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';

const read = path => readFileSync(path, 'utf8');
const run = (command, ...args) => execFileSync(command, args, { encoding: 'utf8' }).trim();
const nativeDir = 'packages/webfont-generator';
const engineDir = 'crates/webfont-generator';
const version = dir => JSON.parse(read(`${dir}/package.json`)).version;
const packageVersion = /(\[package\][\s\S]*?\nversion = ")([^"]+)(")/;
const engineDependency = /(^webfont-generator = \{[^\n]*version = ")([^"]+)(")/m;
const cargoVersion = dir => read(`${dir}/Cargo.toml`).match(packageVersion)?.[2];
const packages = [
    ['native', nativeDir, 'webfont-generator'],
    ['engine', engineDir, 'webfont-engine'],
    ['plugin', 'packages/vite-svg-2-webfont', 'vite-svg-2-webfont'],
    ['browser', 'packages/webfont-browser', 'webfont-browser-generator'],
];

function check() {
    assert.equal(cargoVersion(engineDir), version(nativeDir), 'Engine/native versions must match');
    assert.equal(cargoVersion(nativeDir), version(nativeDir), 'NAPI Cargo/npm versions must match');
    assert.equal(read(`${nativeDir}/Cargo.toml`).match(engineDependency)?.[2], version(nativeDir), 'NAPI engine dependency must match');
}

function section(dir, current) {
    const notes = read(`${dir}/CHANGELOG.md`)
        .split(/(?=^## )/m)
        .find(part => part.startsWith(`## ${current}\n`));
    assert.ok(notes, `Missing pnpm changelog section for ${dir}@${current}`);
    return notes.trim();
}

function sync() {
    const current = version(nativeDir);
    assert.match(current, /^\d+\.\d+\.\d+$/, 'Releases currently use stable versions');
    if (cargoVersion(engineDir) !== current) {
        const notes = section(nativeDir, current);
        const path = `${engineDir}/CHANGELOG.md`;
        const existing = read(path);
        const heading = existing.match(/^# .*\n+/)?.[0] ?? '';
        writeFileSync(path, `${heading}${notes}\n\n${existing.slice(heading.length)}`);
    }
    for (const [dir, patterns] of Object.entries({
        [engineDir]: [packageVersion],
        [nativeDir]: [packageVersion, engineDependency],
    })) {
        const path = `${dir}/Cargo.toml`;
        let text = read(path);
        for (const pattern of patterns) {
            assert.ok(pattern.test(text), `Missing version field in ${path}`);
            text = text.replace(pattern, (_, prefix, _old, suffix) => `${prefix}${current}${suffix}`);
        }
        writeFileSync(path, text);
    }
    check();
}

function plan(base) {
    assert.match(base ?? '', /^[a-f0-9]{40}$/, 'Expected the push base SHA');
    check();
    const changed = run('git', 'diff', '--name-only', base, 'HEAD').split('\n');
    const releases = [];
    for (const [id, dir, component] of packages) {
        const manifest = `${dir}/${id === 'engine' ? 'Cargo.toml' : 'package.json'}`;
        const previous = run('git', 'show', `${base}:${manifest}`);
        const before = id === 'engine' ? previous.match(packageVersion)?.[2] : JSON.parse(previous).version;
        const current = id === 'engine' ? cargoVersion(dir) : version(dir);
        if (before === current) continue;
        assert.ok(changed.includes('.changeset/ledger.yaml'), 'Version changes require a pnpm release ledger update');
        assert.match(current, /^\d+\.\d+\.\d+$/, 'Releases currently use stable versions');
        releases.push({ id, tag: `${component}-v${current}`, notes: section(dir, current) });
    }
    return releases;
}

const [command, base] = process.argv.slice(2);
switch (command) {
    case 'check':
        check();
        break;
    case 'sync':
        sync();
        break;
    case 'plan':
        console.log(JSON.stringify(plan(base)));
        break;
    case 'publish': {
        // Validate the entire plan before creating any tags or GitHub releases.
        const releases = plan(base);
        const head = run('git', 'rev-parse', 'HEAD');
        for (const { tag, notes } of releases) {
            const tags = run('git', 'tag', '--list', tag);
            if (tags) {
                assert.equal(run('git', 'rev-list', '-n', '1', tag), head, `Tag ${tag} points at another commit`);
            } else {
                run('git', 'tag', tag, head);
                run('git', 'push', 'origin', `refs/tags/${tag}`);
            }
            // A rerun reuses the release and retries the existing publication jobs.
            const existing = run('gh', 'api', `repos/${process.env.GITHUB_REPOSITORY}/releases`, '--paginate', '--jq', '.[].tag_name').split('\n');
            if (!existing.includes(tag)) run('gh', 'release', 'create', tag, '--verify-tag', '--title', tag, '--notes', notes);
        }
        for (const [id] of packages) {
            const release = releases.find(item => item.id === id);
            appendFileSync(process.env.GITHUB_OUTPUT, `${id}-released=${Boolean(release)}\n${id}-tag=${release?.tag ?? ''}\n`);
        }
        appendFileSync(process.env.GITHUB_OUTPUT, `any-released=${releases.length > 0}\n`);
        break;
    }
    default:
        throw new Error('Usage: node scripts/release.mjs check|sync|plan BASE|publish BASE');
}
