export const packageNames = {
    engine: '@atlowchemi/webfont-engine',
    adapter: '@atlowchemi/webfont-generator',
    plugin: 'vite-svg-2-webfont',
    docs: '@atlowchemi/vite-svg-webfont-docs',
    example: '@atlowchemi/vite-svg-webfont-example',
    root: '@atlowchemi/vite-svg-webfont-mono',
} as const;

const packageDirectories = ['crates/webfont-generator/', 'packages/webfont-generator/', 'packages/vite-svg-2-webfont/', 'packages/docs/', 'packages/example/'];
const jobNames = ['ci', 'rust-coverage', 'native-coverage', 'build', 'docs', 'test-browser', 'test-host', 'test-docker', 'test-vite-compat'] as const;
type Job = (typeof jobNames)[number];
type Suite = 'engine' | 'cli' | 'adapter';
export type Selection = {
    schemaVersion: 1;
    mode: 'affected';
    full: boolean;
    reasons: string[];
    changedFiles: string[];
    affectedPackages: string[];
    jobs: Record<Job, { selected: boolean; reasons: string[] }>;
    rustSuites: Suite[];
};

export function selectJobs(changedFiles: string[], affectedPackages: string[], fullReason?: string): Selection {
    const jobs = Object.fromEntries(jobNames.map(name => [name, { selected: false, reasons: [] as string[] }])) as Selection['jobs'];
    const reasons: string[] = [];
    const suites = new Set<Suite>();
    const select = (job: Job, reason: string) => {
        jobs[job].selected = true;
        if (!jobs[job].reasons.includes(reason)) jobs[job].reasons.push(reason);
    };
    const fullReasons = [
        ...(fullReason ? [fullReason] : []),
        ...changedFiles.filter(path => !packageDirectories.some(directory => path.startsWith(directory))).map(path => `Shared or unclassified path: ${path}`),
        ...changedFiles.filter(path => /(?:^|\/)(?:package\.json|Cargo\.toml)$/.test(path)).map(path => `Workspace graph/manifest changed: ${path}`),
        ...affectedPackages.filter(name => !Object.values<string>(packageNames).includes(name)).map(name => `Unclassified package: ${name}`),
    ];
    if (changedFiles.length > 0 && affectedPackages.length === 0) fullReasons.push('Changed files produced no affected packages');
    if (fullReasons.length > 0) {
        reasons.push(...fullReasons);
        for (const job of jobNames) for (const reason of fullReasons) select(job, reason);
        suites.add('engine').add('cli').add('adapter');
    } else {
        select('ci', 'Always run shared lint and Rust checks');
        const components = new Set(affectedPackages);
        // Reverse file consumers are not represented by npm dependency edges.
        const sharedFixtures = changedFiles.filter(
            path => path.startsWith('packages/webfont-generator/templates/') || path.startsWith('packages/vite-svg-2-webfont/src/fixtures/'),
        );
        if (sharedFixtures.length) {
            components.add(packageNames.engine);
            components.add(packageNames.adapter);
            components.add(packageNames.plugin);
            reasons.push(`Engine tests consume templates/SVG fixtures: ${sharedFixtures.join(', ')}`);
        }
        if (changedFiles.some(path => path === 'crates/webfont-generator/CHANGELOG.md' || path === 'packages/webfont-generator/CHANGELOG.md')) {
            components.add(packageNames.docs);
            select('docs', 'Docs embed the changed engine/adapter changelog');
        }
        if (components.has(packageNames.engine)) {
            suites.add('engine').add('cli');
            select('rust-coverage', 'Engine or shared engine test input changed');
        }
        if (components.has(packageNames.adapter) || components.has(packageNames.engine)) {
            // Engine and adapter share the existing rust-tests Codecov flag.
            // Refresh both together; a partial replacement would lose flag data.
            suites.add('engine').add('cli').add('adapter');
            for (const job of ['rust-coverage', 'native-coverage', 'test-host', 'test-docker'] as const) select(job, 'Engine/adapter validation');
        }
        if ([packageNames.engine, packageNames.adapter, packageNames.plugin, packageNames.example].some(name => components.has(name))) {
            select('test-vite-compat', 'Affected native/plugin consumer; existing matrix also runs integration tests');
            select('test-browser', 'Affected font/plugin consumer; existing job runs both browser suites');
        }
        if (components.has(packageNames.docs)) select('docs', 'Affected docs package or included changelog');
        if (jobs['test-host'].selected || jobs['test-docker'].selected || jobs['test-vite-compat'].selected) {
            select('build', 'Prerequisite: selected host/musl/Vite jobs download native binding artifacts');
        }
    }
    return {
        schemaVersion: 1,
        mode: 'affected',
        full: fullReasons.length > 0,
        reasons,
        changedFiles: [...new Set(changedFiles)].toSorted(),
        affectedPackages: [...new Set(affectedPackages)].toSorted(),
        jobs,
        rustSuites: (['engine', 'cli', 'adapter'] as const).filter(suite => suites.has(suite)),
    };
}

// --no-renames emits both sides as deletion/addition. NUL delimiters preserve
// spaces, tabs, and newlines in paths without parsing Git's quoted display form.
export function parseChangedFiles(output: string): string[] {
    return output.split('\0').filter(Boolean);
}

export function parsePackages(output: string): string[] {
    const projects: unknown = JSON.parse(output);
    if (!Array.isArray(projects) || projects.some(project => !project || typeof project.name !== 'string')) throw new Error('Invalid pnpm package-list JSON');
    return projects.map(project => project.name);
}

export type CommandRunner = (command: string, args: string[]) => string;
export type Comparison = { eventName: string; head: string; base?: string; full?: boolean };
export type SelectionReport = Selection & {
    comparison: { event: string; head: string; requestedBase: string | null; mergeBase: string | null };
};
const validSha = (sha: string | undefined) => sha && /^[a-f0-9]{40}$/.test(sha) && !/^0+$/.test(sha);
const escape = (text: string) => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('|', '&#124;').replaceAll('\n', ' ');

export function resolveSelection(comparison: Comparison, run: CommandRunner): SelectionReport {
    let base: string | null = null;
    let changedFiles: string[] = [];
    let affectedPackages: string[] = [];
    let selection: Selection;
    try {
        if (comparison.full || !['pull_request', 'push'].includes(comparison.eventName)) {
            selection = selectJobs([], [], `Full validation requested for ${comparison.eventName}`);
        } else {
            if (!validSha(comparison.head) || !validSha(comparison.base)) throw new Error('Missing or invalid comparison revision');
            if (run('git', ['rev-parse', 'HEAD']).trim() !== comparison.head) throw new Error('Checkout does not match event head');
            base = run('git', ['merge-base', comparison.base!, comparison.head]).trim();
            if (!validSha(base)) throw new Error('Invalid merge-base');
            if (comparison.eventName === 'push' && base !== comparison.base) throw new Error('Push rewrote history; comparison requires full validation');
            changedFiles = parseChangedFiles(run('git', ['diff', '--name-only', '--no-renames', '-z', base, comparison.head, '--']));
            affectedPackages = parsePackages(run('vp', ['exec', 'pnpm', '--filter', `...[${base}]`, 'list', '--depth', '-1', '--json']));
            selection = selectJobs(changedFiles, affectedPackages);
        }
    } catch (error) {
        selection = selectJobs(changedFiles, affectedPackages, `Selection failed; use full validation: ${error instanceof Error ? error.message : String(error)}`);
    }
    return { ...selection, comparison: { event: comparison.eventName, head: comparison.head, requestedBase: comparison.base ?? null, mergeBase: base } };
}

export function selectionSummary(selection: ReturnType<typeof resolveSelection>): string {
    return [
        '## Affected CI selection',
        '',
        'Selected jobs must succeed; only jobs explicitly not selected may be skipped.',
        '',
        `- Head: ${escape(selection.comparison.head)}`,
        `- Merge-base: ${selection.comparison.mergeBase ?? 'unavailable/not needed'}`,
        `- Full selection: ${selection.full}`,
        `- pnpm affected packages: ${selection.affectedPackages.map(escape).join(', ') || '(none)'}`,
        `- Rust suites: ${selection.rustSuites.join(', ') || '(none)'}`,
        ...selection.reasons.map(reason => `- ${escape(reason)}`),
        '',
        '| Job | Selected | Reason |',
        '| --- | --- | --- |',
        ...Object.entries(selection.jobs).map(([job, value]) => `| ${job} | ${value.selected ? 'yes' : 'no'} | ${value.reasons.map(escape).join('; ') || 'No affected input'} |`),
        '',
        'Changed paths and full machine-readable evidence are in the `affected-selection` artifact.',
        '',
    ].join('\n');
}
