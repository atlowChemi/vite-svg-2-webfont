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
        .find(part => part.startsWith(`## ${current}\n`) || part.startsWith(`## [${current}](`));
    assert.ok(notes, `Missing pnpm changelog section for ${dir}@${current}`);
    return notes.trim();
}

function sync() {
    const current = version(nativeDir);
    assert.match(current, /^\d+\.\d+\.\d+$/, 'Releases currently use stable versions');
    if (cargoVersion(engineDir) !== current) {
        // The engine gets its own compare link, even if native notes are already formatted.
        const notes = section(nativeDir, current).replace(/^## [^\n]+/, `## ${current}`);
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
    const date = new Date().toISOString().slice(0, 10);
    for (const [id, dir, component] of packages) {
        const engine = id === 'engine';
        const manifest = run('git', 'show', `HEAD:${dir}/${engine ? 'Cargo.toml' : 'package.json'}`);
        const before = engine ? manifest.match(packageVersion)?.[2] : JSON.parse(manifest).version;
        const next = engine ? cargoVersion(dir) : version(dir);
        if (before === next) continue;
        assert.match(before, /^\d+\.\d+\.\d+$/);
        assert.match(next, /^\d+\.\d+\.\d+$/);
        const path = `${dir}/CHANGELOG.md`;
        const text = read(path);
        const notes = section(dir, next);
        const heading = `## [${next}](https://github.com/atlowChemi/vite-svg-2-webfont/compare/${component}-v${before}...${component}-v${next}) (${date})`;
        // Only decorate the newly generated plain heading; preserve history and rerun dates.
        if (notes.split('\n')[0] === `## ${next}`) {
            writeFileSync(path, text.replace(notes, notes.replace(/^## [^\n]+/, heading)));
        }
    }
}

function plan(base) {
    assert.match(base ?? '', /^[a-f0-9]{40}$/, 'Expected the push base SHA');
    check();
    const changed = run('git', 'diff', '--name-only', base).split('\n');
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

async function requireIntent(base) {
    assert.match(base ?? '', /^[a-f0-9]{40}$/, 'Expected the PR base SHA');
    const mergeBase = run('git', 'merge-base', base, 'HEAD');
    const added = run('git', 'diff', '--name-only', '--diff-filter=A', '--no-renames', '-z', mergeBase, 'HEAD', '--', '.changeset/')
        .split('\0')
        .filter(path => /^\.changeset\/[^/]+\.md$/.test(path) && path !== '.changeset/README.md');
    assert.ok(added.length, 'Every PR must add a new intent: run pnpm change (use --bump none for no release).');
    const { parse } = await import('yaml');
    for (const path of added) {
        const match = read(path).match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/);
        assert.ok(match, `Invalid intent frontmatter: ${path}`);
        const releases = parse(match[1]);
        assert.ok(releases && typeof releases === 'object' && !Array.isArray(releases), `Intent must name packages: ${path}`);
        const bumps = Object.values(releases);
        assert.ok(bumps.length && bumps.every(bump => ['none', 'patch', 'minor', 'major'].includes(bump)), `Invalid intent bumps: ${path}`);
        assert.ok(match[2].trim(), `Intent needs a summary, including for no-release decisions: ${path}`);
    }
}

const [command, base] = process.argv.slice(2);
switch (command) {
    case 'version': {
        const applied = JSON.parse(run('pnpm', 'version', '-r', '--json'));
        const unbumped = applied.filter(release => release.currentVersion === release.newVersion);
        assert.equal(
            unbumped.length,
            0,
            `Requested releases did not advance: ${unbumped.map(release => release.name).join(', ')}. Publish/approve their preceding npm versions before preparing a release.`,
        );
        console.log(JSON.stringify(applied));
        break;
    }
    case 'intent':
        await requireIntent(base);
        break;
    case 'prepare-check': {
        const head = run('git', 'rev-parse', 'HEAD');
        assert.ok(plan(head).length, 'No versions advanced. Check whether the preceding npm release is publicly available.');
        break;
    }
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
        throw new Error('Usage: node scripts/release.mjs version|check|sync|intent BASE|prepare-check|plan BASE|publish BASE');
}
