import { resolve, relative, isAbsolute, sep } from 'node:path';
import type { GlyphChangeEntry, RegenerationFiles } from '@atlowchemi/webfont-generator';
import type { IconPluginOptions, ParsedOptions } from './optionParser';
import type { WatchedChangeBatch } from './utils';

type QueueWorkItem = () => Promise<void>;
interface Queue {
    /** Schedule work to run in the serialized queue. */
    run(work: QueueWorkItem): Promise<void>;
    /** Wait for all scheduled work to complete. */
    idle(): Promise<void>;
}

/** One serialized lineage per plugin, including startup and full-build fallbacks. */
export function createGenerationQueue(signal: AbortSignal): Queue {
    let pending = Promise.resolve();
    return {
        run(work: QueueWorkItem): Promise<void> {
            const next = pending.then(() => (signal.aborted ? undefined : work()));
            pending = next.catch(() => undefined);
            return next;
        },
        idle: () => pending,
    };
}

export function regenerationFiles(options: Pick<ParsedOptions, 'files' | 'variants'>): RegenerationFiles {
    if (options.variants) {
        return { variants: options.variants.map(variant => ({ variant: variant.name, files: variant.files })) };
    }
    return { files: options.files! };
}

function familyPaths(inputs: RegenerationFiles) {
    return new Set(inputs.variants ? inputs.variants.flatMap(variant => variant.files) : inputs.files);
}

/** Reconcile hints against family membership, not platform-specific rename event labels. */
export function reconcileChanges(before: RegenerationFiles, after: RegenerationFiles, events: WatchedChangeBatch): GlyphChangeEntry[] | undefined {
    const previous = familyPaths(before);
    const current = familyPaths(after);
    const changes = new Map<string, GlyphChangeEntry>();
    for (const path of previous) {
        if (!current.has(path)) changes.set(path, { path, changeType: 'removed' });
    }
    for (const path of current) {
        if (!previous.has(path)) changes.set(path, { path, changeType: 'added' });
    }
    for (const event of events) {
        if (!event.path.endsWith('.svg')) return undefined;
        if (current.has(event.path) && previous.has(event.path)) {
            changes.set(event.path, { path: event.path, changeType: 'changed' });
        }
    }
    return changes.values().toArray();
}

/** Variant contexts are recursive; ancestor coverage subsumes nested/shared roots. */
export function watchRoots(options: IconPluginOptions): string | string[] {
    if (!options.variants) return options.context;
    const roots = new Set(options.variants.map(variant => resolve(options.context, variant.context ?? '.')));
    return roots
        .values()
        .filter(
            root =>
                !roots.values().some(parent => {
                    const path = relative(parent, root);
                    return parent !== root && !isAbsolute(path) && path !== '..' && !path.startsWith(`..${sep}`);
                }),
        )
        .toArray();
}
