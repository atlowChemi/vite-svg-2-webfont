import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { platform, arch, release } from 'node:os';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
if (process.argv[2] === '--require-fingerprint') {
    if (!process.env.CACHE_TOOLCHAIN_FINGERPRINT) throw new Error('Use pnpm run next:pipeline <pipeline> so cache keys include the toolchain');
    process.exit(0);
}
const version = (command, args) => {
    // Match ordinary compiler selections such as CC="ccache cc" without a shell.
    const [program, ...prefix] = existsSync(command) ? [command] : command.trim().split(/\s+/);
    return execFileSync(program, [...prefix, ...args], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
};
// Compute before pnpm chooses cache keys, never from inside a cached task.
const identity = {
    platform: platform(),
    arch: arch(),
    release: release(),
    node: process.version,
    pnpm: version('pnpm', ['--version']),
    rustc: version(process.env.RUSTC || 'rustc', ['-vV']),
    cargo: version('cargo', ['--version']),
    cc: version(process.env.CC || 'cc', ['--version']),
    cxx: version(process.env.CXX || 'c++', ['--version']),
    cargoConfig: createHash('sha256')
        .update(readFileSync(new URL('../.cargo/config.toml', import.meta.url)))
        .digest('hex'),
    buildEnvironment: Object.fromEntries(
        Object.entries(process.env)
            .filter(([name]) =>
                /^(CARGO_(BUILD|PROFILE|TARGET|ENCODED|INCREMENTAL)|RUSTFLAGS$|RUSTUP_TOOLCHAIN$|RUSTC($|_)|CC($|_)|CXX($|_)|AR$|LD$|SDKROOT$|MACOSX_DEPLOYMENT_TARGET$|WASM_BINDGEN$)/.test(
                    name,
                ),
            )
            .toSorted(([a], [b]) => a.localeCompare(b)),
    ),
};
if (process.platform === 'linux') identity.libc = version('getconf', ['GNU_LIBC_VERSION']);
const fingerprint = createHash('sha256').update(JSON.stringify(identity)).digest('hex');
const fileInputs = {};
for (const [variable, path] of [
    ['CACHE_INPUT_EXAMPLE_ENV_LOCAL', 'packages/example/.env.local'],
    ['CACHE_INPUT_EXAMPLE_ENV_PRODUCTION_LOCAL', 'packages/example/.env.production.local'],
]) {
    try {
        fileInputs[variable] = createHash('sha256')
            .update(readFileSync(new URL(`../${path}`, import.meta.url)))
            .digest('hex');
    } catch (error) {
        if (error.code !== 'ENOENT') throw error;
        fileInputs[variable] = 'missing';
    }
}
if (process.argv[2] === '--fingerprint') {
    console.log(JSON.stringify({ fingerprint, identity, fileInputs }, null, 2));
} else {
    const pipeline = process.argv[2];
    if (!pipeline?.startsWith('next-')) throw new Error('Expected a next-* pipeline name');
    const result = spawnSync('pnpm', ['pipeline', pipeline, '--full', ...process.argv.slice(3)], {
        cwd: root,
        stdio: 'inherit',
        env: { ...process.env, ...fileInputs, CACHE_TOOLCHAIN_FINGERPRINT: fingerprint },
    });
    if (result.error) throw result.error;
    process.exit(result.status ?? 1);
}
