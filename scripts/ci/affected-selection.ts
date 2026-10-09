import process from 'node:process';
import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export const packageNames = {
    engine: '@atlowchemi/webfont-engine',
    adapter: '@atlowchemi/webfont-generator',
    plugin: 'vite-svg-2-webfont',
    docs: '@atlowchemi/vite-svg-webfont-docs',
    example: '@atlowchemi/vite-svg-webfont-example',
    root: '@atlowchemi/vite-svg-webfont-mono',
} as const;
const allPackages = Object.values<string>(packageNames);
const changelogFiles = new Set(['crates/webfont-generator/CHANGELOG.md', 'packages/webfont-generator/CHANGELOG.md', 'packages/vite-svg-2-webfont/CHANGELOG.md']);

// Map pnpm's affected packages to the existing CI job groups and prerequisites.
export function selectJobs(packages: string[], files: string[], unknownChanges = false): Record<string, boolean> {
    const native = packages.includes(packageNames.engine) || packages.includes(packageNames.adapter);
    const plugin = native || packages.includes(packageNames.plugin) || packages.includes(packageNames.example);
    return {
        ci: true,
        'test-scripts': unknownChanges || files.some(file => file.startsWith('scripts/')),
        'rust-coverage': native,
        'native-coverage': native,
        build: plugin,
        docs: packages.includes(packageNames.docs),
        'test-browser': plugin,
        'test-host': native,
        'test-docker': native,
        'platform-output': native,
        'test-vite-compat': plugin,
    };
}

export type Selection = {
    base: string | null;
    packages: string[];
    files: string[];
    full: boolean;
    reason: string;
    jobs: Record<string, boolean>;
    rustSuites: ('engine' | 'cli' | 'adapter')[];
    nativeBuildScope: 'full' | 'linux-x64';
};
type CommandRunner = (command: string, args: string[]) => string;
const execute: CommandRunner = (command, args) => execFileSync(command, args, { encoding: 'utf8', timeout: 60_000, maxBuffer: 8 * 1024 * 1024 });

export function determineValidationScope(files: string[], affected: ReadonlySet<string>): { full: boolean; reason: string } {
    const unknown = affected
        .values()
        .filter(name => !allPackages.includes(name))
        .toArray();
    if (unknown.length) {
        throw new Error(`Unknown affected packages: ${unknown.toSorted().join(', ')}. Update the affected-selection package and job mapping.`);
    }
    if (files.some(file => !/^(crates\/webfont-generator|packages\/(webfont-generator|vite-svg-2-webfont|docs|example))\//.test(file))) {
        return { full: true, reason: 'Shared or unclassified files changed: validate all packages' };
    }
    if (files.length > 0 && (affected.size === 0 || (affected.size === 1 && affected.has(packageNames.root)))) {
        return { full: true, reason: 'Changed files lack an affected package selection: validate all packages' };
    }
    return { full: false, reason: 'Changed packages and their dependents (including shared file consumers)' };
}

export function resolveSelection(baseSha: string | undefined, run: CommandRunner = execute, push = false): Selection {
    let packages = allPackages;
    let files: string[] = [];
    let base: string | null = null;
    let full = true;
    let unknownChanges = false;
    let reason = 'Validate all packages';
    let affected: Set<string> | undefined;
    try {
        if (!baseSha || !/^[a-f0-9]{40}$/.test(baseSha) || /^0+$/.test(baseSha)) {
            throw new Error('Missing comparison revision');
        }
        base = run('git', ['merge-base', baseSha, 'HEAD']).trim();
        if (!/^[a-f0-9]{40}$/.test(base)) {
            throw new Error('Invalid merge-base');
        }
        if (push && base !== baseSha) {
            throw new Error('Push rewrote history');
        }
        // Both sides of renames are needed for ownership; NUL preserves unusual paths.
        files = run('git', ['diff', '--name-only', '--no-renames', '-z', base, 'HEAD', '--']).split('\0').filter(Boolean);
        const filters = [`...[${base}]`];
        // Docs include engine/adapter release notes and symlink the plugin changelog.
        if (files.some(file => changelogFiles.has(file))) {
            filters.push(packageNames.docs);
        }
        const projects: unknown = JSON.parse(run('vp', ['exec', 'pnpm', 'list', '--depth', '-1', '--json', ...filters.flatMap(filter => ['--filter', filter])]));
        if (!Array.isArray(projects) || projects.some(project => !project || typeof project.name !== 'string')) {
            throw new Error('Invalid pnpm package list');
        }
        affected = new Set<string>(projects.map(project => project.name));
    } catch (error) {
        unknownChanges = true;
        reason = `Selection unavailable: validate everything. ${error instanceof Error ? error.message : String(error)}`;
    }
    // Mapping errors must fail CI, rather than being swallowed by the comparison fallback.
    if (affected) {
        ({ full, reason } = determineValidationScope(files, affected));
        packages = full ? allPackages : [...affected];
    }
    const jobs = selectJobs(packages, files, unknownChanges);
    return {
        base,
        files,
        packages: packages.toSorted(),
        full,
        reason,
        jobs,
        rustSuites: packages.includes(packageNames.engine) ? ['engine', 'cli', 'adapter'] : packages.includes(packageNames.adapter) ? ['adapter'] : [],
        nativeBuildScope: jobs['test-host'] ? 'full' : 'linux-x64',
    };
}

const { argv, env } = process;

const executedFile = argv[1];
const currentModuleAbsoluteUrl = import.meta.url;
const { href: executedFileHref } = (executedFile && pathToFileURL(executedFile)) || { href: '' };

if (executedFile && currentModuleAbsoluteUrl === executedFileHref) {
    let event;
    try {
        event = JSON.parse(readFileSync(env.GITHUB_EVENT_PATH ?? '', 'utf8'));
    } catch {
        /* Missing event data falls back to full validation. */
    }
    try {
        const selection = resolveSelection(event?.pull_request?.base?.sha ?? event?.before, execute, env.GITHUB_EVENT_NAME === 'push');
        const json = JSON.stringify(selection, null, 2);

        mkdirSync('artifacts/affected-selection', { recursive: true });
        writeFileSync('artifacts/affected-selection/selection.json', `${json}\n`);

        if (env.GITHUB_OUTPUT) {
            appendFileSync(env.GITHUB_OUTPUT, `selection=${JSON.stringify(selection)}\n`);
        }
        if (env.GITHUB_STEP_SUMMARY) {
            appendFileSync(env.GITHUB_STEP_SUMMARY, `## Affected validation\n\n\`\`\`json\n${json.replaceAll('`', '\\u0060')}\n\`\`\`\n`);
        }
        console.log(json);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`::error title=Affected selection failed::${message.replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A')}`);
        process.exitCode = 1;
    }
}
