import { spawn } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const binary = process.env.CACHE_AUDIT_BIN || join(root, 'crates/cache-audit/target/debug/cache-audit');
const [command, ...options] = process.argv.slice(2);
let args = options;
// Preserve the existing audit TASK [POLICY] entry point.
if (command === 'audit' && args[0] && !args[0].startsWith('--')) {
    const [task, policy, ...rest] = args;
    if (rest.length) throw new Error('Expected TASK [POLICY]');
    args = ['--task', task, ...(policy ? ['--policy', policy] : [])];
}
const child = spawn(binary, [command, '--workspace', root, ...args], { cwd: root, stdio: 'inherit' });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.on('error', error => {
    console.error(error);
    process.exitCode = 1;
});
child.on('exit', (code, signal) => {
    process.exitCode = code ?? (signal === 'SIGINT' ? 130 : 143);
});
