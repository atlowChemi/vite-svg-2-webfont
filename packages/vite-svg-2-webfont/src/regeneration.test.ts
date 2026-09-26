import { describe, expect, it, vi, beforeEach, afterEach } from 'vite-plus/test';
import { createGenerationQueue, reconcileChanges, watchRoots } from './regeneration';
import { resolve, relative, isAbsolute, posix, win32 } from 'node:path';
import type { IconPluginOptions } from './optionParser';
import type { GlyphChangeEntry } from '@atlowchemi/webfont-generator';
import type { WatchedChangeBatch } from './utils';

const separator = vi.hoisted(() => vi.fn<() => typeof import('node:path').sep>());
vi.mock('node:path', async importOriginal => {
    const actual = await importOriginal<typeof import('node:path')>();
    separator.mockReturnValue(actual.sep);
    return {
        ...actual,
        resolve: vi.fn((...paths: string[]) => actual.resolve(...paths)),
        relative: vi.fn((from: string, to: string) => actual.relative(from, to)),
        isAbsolute: vi.fn((value: string) => actual.isAbsolute(value)),
        get sep() {
            return separator();
        },
    };
});

function usePaths(paths: typeof posix) {
    vi.mocked(resolve).mockImplementation((...values) => paths.resolve(...values));
    vi.mocked(relative).mockImplementation((from, to) => paths.relative(from, to));
    vi.mocked(isAbsolute).mockImplementation(value => paths.isAbsolute(value));
    separator.mockReturnValue(paths.sep);
}

describe('watchRoots', () => {
    const nativePaths = process.platform === 'win32' ? win32 : posix;
    afterEach(() => {
        vi.mocked(resolve).mockReset();
        vi.mocked(relative).mockReset();
        vi.mocked(isAbsolute).mockReset();
        separator.mockReturnValue(nativePaths.sep);
    });

    describe.each([
        { platform: 'POSIX', paths: posix, root: '/project/icons', external: '/external/designs' },
        { platform: 'Windows', paths: win32, root: 'C:\\project\\icons', external: 'D:\\external\\designs' },
    ])('$platform', ({ paths, root, external }) => {
        beforeEach(() => {
            usePaths(paths);
        });

        it('returns ordinary context verbatim without normalizing its shape', () => {
            expect(watchRoots({ context: './icons/../icons', files: '*.svg' })).toBe('./icons/../icons');
        });
        it('inherits top-level context and deduplicates equivalent paths', () => {
            expect(
                watchRoots({
                    context: root,
                    variants: [
                        { name: 'inherited', default: true },
                        { name: 'dot', context: '.' },
                        { name: 'normalized', context: paths.join('nested', '..') },
                        { name: 'absolute', context: `${root}${paths.sep}` },
                    ],
                }),
            ).toEqual([root]);
        });
        it.each([false, true])('keeps only the ancestor regardless of configuration order (ancestor first=%s)', ancestorFirst => {
            const variants = [
                { name: 'child', context: paths.join('light', 'nested'), default: true },
                { name: 'parent', context: 'light' },
                { name: 'grandchild', context: paths.join('light', 'nested', 'deep') },
            ];
            expect(watchRoots({ context: root, variants: ancestorFirst ? [variants[1]!, variants[0]!, variants[2]!] : variants })).toEqual([paths.join(root, 'light')]);
        });
        it('retains sibling prefixes and similarly named parent directories', () => {
            expect(
                watchRoots({
                    context: root,
                    variants: [
                        { name: 'light', context: 'light', default: true },
                        { name: 'lightest', context: 'lightest' },
                        { name: 'sibling', context: paths.join('..', 'icons-other') },
                        { name: 'dot-prefix', context: '..designs' },
                    ],
                }),
            ).toEqual([paths.join(root, 'light'), paths.join(root, 'lightest'), paths.resolve(root, '..', 'icons-other'), paths.join(root, '..designs')]);
        });
        it('allows a parent context to subsume the top-level context', () => {
            expect(
                watchRoots({
                    context: root,
                    variants: [
                        { name: 'inherited', default: true },
                        { name: 'parent', context: '..' },
                    ],
                }),
            ).toEqual([paths.dirname(root)]);
        });
        it('keeps external roots and stable order among surviving roots', () => {
            const options: IconPluginOptions = {
                context: root,
                variants: [
                    { name: 'external', context: external, default: true },
                    { name: 'child', context: 'light' },
                    { name: 'parent', context: '.' },
                    { name: 'external-child', context: paths.join(external, 'nested') },
                ],
            };
            const snapshot = structuredClone(options);
            expect(watchRoots(options)).toEqual([external, root]);
            expect(options).toEqual(snapshot);
        });
        it('handles the filesystem root as an ancestor', () => {
            const filesystemRoot = paths.parse(root).root;
            expect(
                watchRoots({
                    context: root,
                    variants: [
                        { name: 'child', default: true },
                        { name: 'root', context: filesystemRoot },
                    ],
                }),
            ).toEqual([filesystemRoot]);
        });
        it('returns no roots for an empty list (configuration validation is separate)', () => {
            expect(watchRoots({ context: root, variants: [] })).toEqual([]);
        });
    });

    it('keeps separate UNC shares and removes descendants only within the same share', () => {
        usePaths(win32);
        expect(
            watchRoots({
                context: 'C:\\project\\icons',
                variants: [
                    { name: 'share-child', context: '\\\\server\\icons\\light', default: true },
                    { name: 'share', context: '\\\\server\\icons\\' },
                    { name: 'other-share', context: '\\\\server\\other\\' },
                    { name: 'other-server', context: '\\\\other\\icons\\' },
                ],
            }),
        ).toEqual(['\\\\server\\icons\\', '\\\\server\\other\\', '\\\\other\\icons\\']);
    });
});

const family = (light: string[], bold: string[]) => ({
    variants: [
        { variant: 'light', files: light },
        { variant: 'bold', files: bold },
    ],
});

describe('reconcileChanges', () => {
    it('does nothing for unchanged membership without events', () => {
        expect(reconcileChanges(family(['a.svg'], ['b.svg']), family(['a.svg'], ['b.svg']), [])).toEqual([]);
        expect(reconcileChanges(family([], []), family([], []), [])).toEqual([]);
    });

    it('infers additions and removals even when notifications are missing', () => {
        expect(reconcileChanges(family(['old.svg'], []), family([], ['new.svg']), [])).toEqual([
            { path: 'old.svg', changeType: 'removed' },
            { path: 'new.svg', changeType: 'added' },
        ]);
    });

    it.each(['added', 'changed', 'removed'] as const)('keeps authoritative membership when the event says %s', kind => {
        const events: WatchedChangeBatch = ['new.svg', 'gone.svg', 'retained.svg', 'unmatched.svg'].map(path => ({ path, kind }));
        expect(reconcileChanges(family(['gone.svg', 'retained.svg'], []), family(['new.svg', 'retained.svg'], []), events)).toEqual([
            { path: 'gone.svg', changeType: 'removed' },
            { path: 'new.svg', changeType: 'added' },
            { path: 'retained.svg', changeType: 'changed' },
        ]);
    });

    it('does not report a removal while another design still consumes the path', () => {
        expect(reconcileChanges(family(['shared.svg'], ['shared.svg']), family([], ['shared.svg']), [])).toEqual([]);
    });

    it('loads a new membership of an existing family path without an added hint', () => {
        expect(reconcileChanges(family(['shared.svg'], []), family(['shared.svg'], ['shared.svg']), [])).toEqual([]);
    });

    it('emits one addition or removal for a path shared by multiple designs', () => {
        const shared = family(['shared.svg'], ['shared.svg']);
        expect(reconcileChanges(family([], []), shared, [])).toEqual([{ path: 'shared.svg', changeType: 'added' }]);
        expect(reconcileChanges(shared, family([], []), [])).toEqual([{ path: 'shared.svg', changeType: 'removed' }]);
    });

    it('deduplicates repeated and conflicting notifications for shared paths', () => {
        const shared = family(['shared.svg'], ['shared.svg']);
        expect(
            reconcileChanges(shared, shared, [
                { path: 'shared.svg', kind: 'removed' },
                { path: 'shared.svg', kind: 'added' },
                { path: 'shared.svg', kind: 'changed' },
                { path: 'shared.svg', kind: 'changed' },
            ]),
        ).toEqual([{ path: 'shared.svg', changeType: 'changed' }]);
    });

    it('ignores notifications for paths outside both source sets, including transient files', () => {
        const inputs = family(['a.svg'], []);
        expect(
            reconcileChanges(inputs, inputs, [
                { path: 'ignored.svg', kind: 'changed' },
                { path: 'transient.svg', kind: 'added' },
                { path: 'transient.svg', kind: 'removed' },
            ]),
        ).toEqual([]);
    });

    it('leaves ordering changes to the authoritative lists rather than inventing content changes', () => {
        const before = family(['a.svg', 'b.svg'], ['c.svg']);
        const after = { variants: family(['b.svg', 'a.svg'], ['c.svg']).variants.toReversed() };
        expect(reconcileChanges(before, after, [])).toEqual([]);
    });

    it.each(['first', 'last'] as const)('uses a full re-diff when an ambiguous event is %s in the batch', position => {
        const ambiguous: WatchedChangeBatch[number] = { path: 'icons/nested', kind: 'changed' };
        const changed: WatchedChangeBatch[number] = { path: 'retained.svg', kind: 'changed' };
        const events = position === 'first' ? [ambiguous, changed] : [changed, ambiguous];
        expect(reconcileChanges(family(['gone.svg', 'retained.svg'], []), family(['new.svg', 'retained.svg'], []), events)).toBeUndefined();
    });

    it('supports flat source inputs using the same membership rules', () => {
        expect(reconcileChanges({ files: ['a.svg', 'gone.svg'] }, { files: ['a.svg', 'new.svg'] }, [{ path: 'a.svg', kind: 'added' }])).toEqual([
            { path: 'gone.svg', changeType: 'removed' },
            { path: 'new.svg', changeType: 'added' },
            { path: 'a.svg', changeType: 'changed' },
        ]);
    });

    it('preserves path identity and does not mutate source lists or event batches', () => {
        const before = family(['icons/A.svg'], []);
        const after = family(['icons/a.svg'], []);
        const events: WatchedChangeBatch = [{ path: 'icons/a.svg', kind: 'changed' }];
        const snapshot = structuredClone({ before, after, events });
        const expected: GlyphChangeEntry[] = [
            { path: 'icons/A.svg', changeType: 'removed' },
            { path: 'icons/a.svg', changeType: 'added' },
        ];
        expect(reconcileChanges(before, after, events)).toEqual(expected);
        expect({ before, after, events }).toEqual(snapshot);
    });
});

describe('family regeneration coordination', () => {
    it('reconciles atomic saves and membership across every final consumer', () => {
        expect(
            reconcileChanges(family(['shared.svg', 'old.svg'], ['shared.svg']), family([], ['shared.svg', 'new.svg']), [
                { path: 'shared.svg', kind: 'added' },
                { path: 'shared.svg', kind: 'removed' },
                { path: 'ignored.svg', kind: 'added' },
            ]),
        ).toEqual([
            { path: 'old.svg', changeType: 'removed' },
            { path: 'new.svg', changeType: 'added' },
            { path: 'shared.svg', changeType: 'changed' },
        ]);
    });

    it('uses authoritative lists for membership-only moves and re-diffs ambiguous directory events', () => {
        expect(reconcileChanges(family(['shared.svg'], []), family([], ['shared.svg']), [])).toEqual([]);
        expect(reconcileChanges(family([], []), family(['new.svg'], []), [{ path: 'nested', kind: 'changed' }])).toBeUndefined();
    });

    it('deduplicates recursive roots without swallowing sibling prefixes', () => {
        expect(
            watchRoots({
                context: '.',
                variants: [
                    { name: 'a', context: 'icons', default: true },
                    { name: 'b', context: 'icons/nested' },
                    { name: 'c', context: 'icons' },
                    { name: 'd', context: 'icons-other' },
                ],
            }),
        ).toEqual([resolve('icons'), resolve('icons-other')]);
    });

    it('serializes startup and updates and continues after failure', async () => {
        const queue = createGenerationQueue(new AbortController().signal);
        const gate = Promise.withResolvers<void>();
        const started = Promise.withResolvers<void>();
        const later = vi.fn(async () => undefined);
        const first = queue.run(async () => {
            started.resolve();
            await gate.promise;
            throw new Error('failed');
        });
        const settled = Promise.allSettled([first, queue.run(later)]);
        try {
            await started.promise;
            expect(later).not.toHaveBeenCalled();
        } finally {
            gate.resolve();
        }
        expect((await settled).map(result => result.status)).toEqual(['rejected', 'fulfilled']);
        expect(later).toHaveBeenCalledOnce();
        await queue.idle();
    });

    it('drops queued work on shutdown but settles the active operation', async () => {
        const ac = new AbortController();
        const queue = createGenerationQueue(ac.signal);
        const gate = Promise.withResolvers<void>();
        const started = Promise.withResolvers<void>();
        const later = vi.fn(async () => undefined);
        const tasks = Promise.all([
            queue.run(async () => {
                started.resolve();
                await gate.promise;
            }),
            queue.run(later),
        ]);
        try {
            await started.promise;
            ac.abort();
        } finally {
            gate.resolve();
        }
        await tasks;
        await queue.idle();
        expect(later).not.toHaveBeenCalled();
    });

    it('keeps separate plugin queues independent', async () => {
        const gate = Promise.withResolvers<void>();
        const first = createGenerationQueue(new AbortController().signal).run(() => gate.promise);
        try {
            const other = vi.fn(async () => undefined);
            await createGenerationQueue(new AbortController().signal).run(other);
            expect(other).toHaveBeenCalledOnce();
        } finally {
            gate.resolve();
            await first;
        }
    });
});
