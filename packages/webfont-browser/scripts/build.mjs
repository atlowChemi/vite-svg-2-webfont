import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { cli, setup } from './setup.mjs';

setup();
const root = fileURLToPath(new URL('../../../', import.meta.url));
const target = process.env.CARGO_TARGET_DIR || `${root}/target/webfont-browser`;
execFileSync('cargo', ['build', '-p', 'webfont-browser', '--target', 'wasm32-unknown-unknown', '--release', '--locked'], {
    cwd: root,
    stdio: 'inherit',
    env: { ...process.env, CARGO_TARGET_DIR: target, CARGO_PROFILE_RELEASE_LTO: 'true', CARGO_PROFILE_RELEASE_CODEGEN_UNITS: '1', CARGO_PROFILE_RELEASE_PANIC: 'abort' },
});
execFileSync(cli, [`${target}/wasm32-unknown-unknown/release/webfont_browser.wasm`, '--target', 'web', '--out-dir', `${root}/packages/webfont-browser/generated`], {
    cwd: root,
    stdio: 'inherit',
});
