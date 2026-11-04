import { appendFileSync, existsSync, globSync, readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { parse } from 'yaml';
import type { Selection } from './affected-selection';

export function selectAuditWork(
    selection: Selection,
    pipelines: Record<string, string[]>,
    scripts: Record<string, string>,
    projects: Record<string, string>,
    auditTasks: string[],
): { workers: { id: number; pipelines: string[]; audits: string[] }[]; selection: Selection } {
    const job = (name: string) => selection.jobs[name] === true;
    const affected = (task: string): boolean => {
        if (selection.full) return true;
        if (!selection.packages.length) return false;
        const directory = scripts[task]?.match(/^pnpm --dir (\S+) run \S+$/)?.[1];
        if (directory) {
            const name = projects[directory];
            return name ? selection.packages.includes(name) : true;
        }
        if (task === 'fmt:rust') return job('rust-coverage') || job('test-wasm');
        if (task === 'coverage:scripts') return job('test-scripts');
        if (['test:js', 'coverage', 'test:compat'].includes(task)) {
            return job('test-scripts') || job('build') || job('test-wasm');
        }
        // New/unrecognized commands must run until their ownership is understood.
        return true;
    };
    const selected = Object.entries(pipelines)
        .filter(([, tasks]) => tasks.some(affected))
        .map(([name]) => name)
        .toSorted();
    const audits = auditTasks.filter(affected).toSorted();
    const workers = Array.from({ length: Math.min(2, Math.max(selected.length, audits.length)) }, (_, id) => ({
        id,
        pipelines: selected.filter((_pipeline, index) => index % 2 === id),
        audits: audits.filter((_task, index) => index % 2 === id),
    }));
    return { workers, selection };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    const selection: Selection = JSON.parse(readFileSync('artifacts/affected-selection/selection.json', 'utf8'));
    const pipelines: Record<string, string[]> = JSON.parse(readFileSync('pipelines.json', 'utf8'));
    const enabled = new Set(readFileSync('selected-pipelines.txt', 'utf8').trim().split('\n').filter(Boolean));
    const { scripts } = JSON.parse(readFileSync('package.json', 'utf8'));
    const projects = Object.fromEntries(
        globSync(['crates/*/package.json', 'packages/*/package.json']).map(file => [file.replace(/\/package.json$/, ''), JSON.parse(readFileSync(file, 'utf8')).name]),
    );
    const policy = 'crates/cache-audit/policy.yaml';
    const tasks = existsSync(policy)
        ? Object.keys(parse(readFileSync(policy, 'utf8')).tasks).map(label => {
              if (!label.startsWith('.#')) throw new Error(`Unsupported audit project: ${label}`);
              return label.slice(2);
          })
        : [];
    const work = selectAuditWork(
        selection,
        process.env.AUDIT_ONLY === 'true' ? {} : Object.fromEntries(Object.entries(pipelines).filter(([name]) => enabled.has(name))),
        scripts,
        projects,
        tasks,
    );
    const json = JSON.stringify(work);
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `work=${json}\n`);
    if (process.env.GITHUB_STEP_SUMMARY)
        appendFileSync(process.env.GITHUB_STEP_SUMMARY, `## Selected audit work\n\n\`\`\`json\n${JSON.stringify(work, null, 2).replaceAll('`', '\\u0060')}\n\`\`\`\n`);
    console.log(json);
}
