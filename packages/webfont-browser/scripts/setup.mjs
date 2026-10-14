import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// Keep the CLI exactly matched to the Rust binding dependency.
const manifest = readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8');
export const version = manifest.match(/^wasm-bindgen = "=([^"]+)"/m)[1];
export const tools = fileURLToPath(new URL('../../../target/wasm-bindgen-tools/', import.meta.url));
export const cli = process.env.WASM_BINDGEN ?? `${tools}/bin/wasm-bindgen${process.platform === 'win32' ? '.exe' : ''}`;

export function setup() {
    const targets = execFileSync('rustup', ['target', 'list', '--installed'], { encoding: 'utf8' });
    if (!targets.split(/\s+/).includes('wasm32-unknown-unknown')) execFileSync('rustup', ['target', 'add', 'wasm32-unknown-unknown'], { stdio: 'inherit' });
    const installed = spawnSync(cli, ['--version'], { encoding: 'utf8' });
    if (installed.status === 0 && installed.stdout.trim() === `wasm-bindgen ${version}`) return;
    if (process.env.WASM_BINDGEN) throw new Error(`WASM_BINDGEN must point to wasm-bindgen ${version}`);
    mkdirSync(tools, { recursive: true });
    execFileSync('cargo', ['install', 'wasm-bindgen-cli', '--version', version, '--locked', '--root', tools], { stdio: 'inherit' });
}
