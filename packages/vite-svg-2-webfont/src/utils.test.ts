import * as fs from 'node:fs/promises';
import { addAbortListener } from 'node:events';
import { join as pathJoin } from 'node:path';
import { setTimeout } from 'node:timers/promises';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import * as utils from './utils';

vi.mock('fs/promises', async () => {
    const fsPromises = await vi.importActual<typeof import('fs/promises')>('fs/promises');
    const access = vi.fn().mockRejectedValueOnce(new Error());
    return { ...fsPromises, access, watch: vi.fn(fsPromises.watch), stat: vi.fn(fsPromises.stat), mkdir: vi.fn(), writeFile: vi.fn() };
});

describe('utils', () => {
    describe('doesFileExist', () => {
        afterEach(() => {
            vi.resetAllMocks();
        });

        it("return false if file doesn't have read access", async () => {
            expect(await utils.doesFileExist('foo', 'bar')).toEqual(false);
            expect(fs.access).toHaveBeenCalledOnce();
        });

        it('return true if file has read access', async () => {
            expect(await utils.doesFileExist('foo', 'bar')).toEqual(true);
            expect(fs.access).toHaveBeenCalledOnce();
        });
    });

    describe('handleWatchEvent', () => {
        afterEach(() => {
            vi.resetAllMocks();
        });

        const validFileName = 'ex.svg';
        const onChange = vi.fn();
        const doesFileExist = vi.fn();

        it("doesn't execute callback for non string events", async () => {
            await utils.handleWatchEvent('', { eventType: 'rename', filename: Buffer.from(validFileName) }, onChange, doesFileExist);
            expect(doesFileExist).not.toHaveBeenCalled();
            expect(onChange).not.toHaveBeenCalled();
        });

        it("doesn't execute callback for non svg files", async () => {
            await utils.handleWatchEvent('', { eventType: 'rename', filename: 'notsvg.png' }, onChange, doesFileExist);
            expect(doesFileExist).not.toHaveBeenCalled();
            expect(onChange).not.toHaveBeenCalled();
        });

        it("emits a 'removed' event for a change event on a missing file", async () => {
            await utils.handleWatchEvent('', { eventType: 'change', filename: validFileName }, onChange, doesFileExist);
            expect(doesFileExist).toHaveBeenCalledOnce();
            expect(onChange).toHaveBeenCalledExactlyOnceWith({ path: validFileName, kind: 'removed' });
        });

        it("emits a 'changed' event for an edited file", async () => {
            doesFileExist.mockResolvedValueOnce(true);
            await utils.handleWatchEvent('', { eventType: 'change', filename: validFileName }, onChange, doesFileExist);
            expect(onChange).toHaveBeenCalledExactlyOnceWith({ path: validFileName, kind: 'changed' });
        });

        it("emits a 'removed' event for a deleted file", async () => {
            await utils.handleWatchEvent('', { eventType: 'rename', filename: validFileName }, onChange, doesFileExist);
            expect(doesFileExist).toHaveBeenCalledOnce();
            expect(onChange).toHaveBeenCalledExactlyOnceWith({ path: validFileName, kind: 'removed' });
        });

        it("emits an 'added' event for a new/renamed file", async () => {
            doesFileExist.mockResolvedValueOnce(true);
            await utils.handleWatchEvent('', { eventType: 'rename', filename: validFileName }, onChange, doesFileExist);
            expect(doesFileExist).toHaveBeenCalledOnce();
            expect(onChange).toHaveBeenCalledExactlyOnceWith({ path: validFileName, kind: 'added' });
        });
    });

    describe('setupWatcher', () => {
        describe('family watcher', () => {
            beforeEach(() => {
                vi.mocked(fs.watch).mockReset();
                vi.mocked(fs.access).mockReset();
            });
            afterEach(() => {
                vi.mocked(fs.stat).mockReset();
                vi.mocked(fs.watch).mockReset();
                vi.mocked(fs.access).mockReset();
                vi.useRealTimers();
            });

            it('rescans dotted directories while ignoring generated-file rename notifications', async () => {
                const actual = await vi.importActual<typeof import('node:fs/promises')>('node:fs/promises');
                vi.mocked(fs.stat).mockResolvedValueOnce(await actual.stat(import.meta.dirname));
                vi.mocked(fs.stat).mockResolvedValueOnce(await actual.stat(import.meta.filename));
                vi.mocked(fs.watch).mockImplementation(() =>
                    (async function* () {
                        yield { eventType: 'rename' as const, filename: 'design.v2' };
                        yield { eventType: 'rename' as const, filename: 'icons.woff2' };
                        return undefined;
                    })(),
                );
                const onChange = vi.fn();
                await utils.setupWatcher(['icons'], new AbortController().signal, onChange);
                expect(fs.stat).toHaveBeenNthCalledWith(1, pathJoin('icons', 'design.v2'));
                expect(fs.stat).toHaveBeenNthCalledWith(2, pathJoin('icons', 'icons.woff2'));
                expect(onChange).toHaveBeenCalledExactlyOnceWith([{ path: 'icons', kind: 'changed' }]);
            });

            it('continues processing SVG edits after a non-SVG path cannot be inspected', async () => {
                vi.mocked(fs.stat).mockRejectedValueOnce(new Error('path disappeared'));
                vi.mocked(fs.watch).mockImplementation(() =>
                    (async function* () {
                        yield { eventType: 'rename' as const, filename: 'icons.css' };
                        yield { eventType: 'change' as const, filename: 'a.svg' };
                        return undefined;
                    })(),
                );
                const onChange = vi.fn();
                await utils.setupWatcher(['icons'], new AbortController().signal, onChange);
                expect(onChange).toHaveBeenCalledExactlyOnceWith([{ path: pathJoin('icons', 'a.svg'), kind: 'changed' }]);
            });

            it('coalesces events across all roots into one family batch', async () => {
                vi.mocked(fs.watch).mockImplementation(root =>
                    (async function* () {
                        yield { eventType: 'change' as const, filename: root === 'light' ? 'a.svg' : 'b.svg' };
                        yield { eventType: 'change' as const, filename: root === 'light' ? 'a.svg' : 'b.svg' };
                        return undefined;
                    })(),
                );
                const onChange = vi.fn();
                await utils.setupWatcher(['light', 'bold'], new AbortController().signal, onChange);
                expect(onChange).toHaveBeenCalledExactlyOnceWith([
                    { path: pathJoin('light', 'a.svg'), kind: 'changed' },
                    { path: pathJoin('bold', 'b.svg'), kind: 'changed' },
                ]);
                expect(fs.watch).toHaveBeenCalledTimes(2);
                expect(fs.watch).toHaveBeenNthCalledWith(1, 'light', expect.objectContaining({ recursive: true }));
                expect(fs.watch).toHaveBeenNthCalledWith(2, 'bold', expect.objectContaining({ recursive: true }));
            });

            it('requests a re-diff for directory and missing-filename notifications', async () => {
                vi.mocked(fs.watch).mockImplementation(() =>
                    (async function* () {
                        yield { eventType: 'rename' as const, filename: 'new-directory' };
                        yield { eventType: 'rename' as const, filename: null };
                        yield { eventType: 'change' as const, filename: 'irrelevant.txt' };
                        return undefined;
                    })(),
                );
                const onChange = vi.fn();
                await utils.setupWatcher(['icons', 'icons'], new AbortController().signal, onChange);
                expect(fs.watch).toHaveBeenCalledExactlyOnceWith('icons', expect.objectContaining({ recursive: true }));
                expect(onChange).toHaveBeenCalledExactlyOnceWith([{ path: 'icons', kind: 'changed' }]);
            });

            it('aborts other root watchers when one root fails', async () => {
                const stopped = vi.fn();
                vi.mocked(fs.watch).mockImplementation((root, options) =>
                    (async function* () {
                        if (root === 'broken') throw new Error('watch failed');
                        if (typeof options !== 'object') throw new Error('Expected watch options');
                        const gate = Promise.withResolvers<void>();
                        addAbortListener(options.signal!, () => gate.resolve());
                        if (options.signal!.aborted) gate.resolve();
                        await gate.promise;
                        stopped();
                        yield* [];
                        return undefined;
                    })(),
                );
                await expect(utils.setupWatcher(['broken', 'other'], new AbortController().signal, vi.fn())).rejects.toThrow('watch failed');
                expect(stopped).toHaveBeenCalledOnce();
            });

            it('uses controlled debounce and deferred handlers to serialize cross-root bursts', async () => {
                vi.useFakeTimers();
                const secondEvent = Promise.withResolvers<void>();
                const stop = Promise.withResolvers<void>();
                const firstBatch = Promise.withResolvers<void>();
                const firstStarted = Promise.withResolvers<void>();
                vi.mocked(fs.watch).mockImplementation(root =>
                    (async function* () {
                        if (root === 'bold') await secondEvent.promise;
                        yield { eventType: 'change' as const, filename: 'a.svg' };
                        await stop.promise;
                        return undefined;
                    })(),
                );
                const onChange = vi.fn().mockImplementationOnce(async () => {
                    firstStarted.resolve();
                    await firstBatch.promise;
                });
                const running = utils.setupWatcher(['light', 'bold'], new AbortController().signal, onChange);
                try {
                    await vi.advanceTimersByTimeAsync(25);
                    await firstStarted.promise;
                    secondEvent.resolve();
                    await vi.advanceTimersByTimeAsync(25);
                    expect(onChange).toHaveBeenCalledExactlyOnceWith([{ kind: 'changed', path: pathJoin('light', 'a.svg') }]);
                } finally {
                    secondEvent.resolve();
                    firstBatch.resolve();
                    stop.resolve();
                    await running;
                }
                expect(onChange).toHaveBeenCalledTimes(2);
            });
        });

        const folderPath = './test-folder';
        const handler = vi.fn();
        let ac: AbortController;

        beforeEach(() => {
            ac = new AbortController();
            handler.mockClear();
        });

        it('throws error if no such folder', async () => {
            const err: unknown = await utils.setupWatcher(folderPath, ac.signal, handler).catch(e => e);
            expect(err).toBeInstanceOf(Error);
            expect((err as Error).message).toBe(`ENOENT: no such file or directory, watch '${folderPath}'`);
        });

        it('handles AbortError without throwing an error', async () => {
            ac.abort();
            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).toEqual(undefined);
        });

        it('batches and coalesces adds and removes', async () => {
            const { watch, access } = fs;
            const event = { eventType: 'rename', filename: 'ex.svg' };
            async function* mock() {
                await Promise.resolve();
                yield event;
                if (vi.isMockFunction(access)) {
                    access.mockRejectedValueOnce(new Error());
                }
                yield event; // file gone -> still surfaces a 'removed' change
                yield event;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }

            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).toEqual(undefined);
            expect(handler).toHaveBeenCalledExactlyOnceWith([{ path: pathJoin(folderPath, 'ex.svg'), kind: 'changed' }]);
        });

        it('batches changes for multiple svg files', async () => {
            const { watch } = fs;
            const events = [
                { eventType: 'change' as const, filename: 'a.svg' },
                { eventType: 'change' as const, filename: 'b.svg' },
            ];
            async function* mock() {
                yield* events;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }

            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).toEqual(undefined);
            expect(handler).toHaveBeenCalledExactlyOnceWith([
                { path: pathJoin(folderPath, 'a.svg'), kind: 'changed' },
                { path: pathJoin(folderPath, 'b.svg'), kind: 'changed' },
            ]);
        });

        it('keeps a newly added svg as added when it changes in the same batch', async () => {
            const { watch } = fs;
            const events = [
                { eventType: 'rename' as const, filename: 'a.svg' },
                { eventType: 'change' as const, filename: 'a.svg' },
            ];
            async function* mock() {
                yield* events;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }

            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).toEqual(undefined);
            expect(handler).toHaveBeenCalledExactlyOnceWith([{ path: pathJoin(folderPath, 'a.svg'), kind: 'added' }]);
        });

        it('keeps the latest change when no special coalescing applies', async () => {
            const { watch } = fs;
            const events = [
                { eventType: 'change' as const, filename: 'a.svg' },
                { eventType: 'change' as const, filename: 'a.svg' },
            ];
            async function* mock() {
                yield* events;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }

            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).toEqual(undefined);
            expect(handler).toHaveBeenCalledExactlyOnceWith([{ path: pathJoin(folderPath, 'a.svg'), kind: 'changed' }]);
        });

        it('serializes async batch handlers', async () => {
            const { watch } = fs;
            const firstHandler = Promise.withResolvers<void>();
            const calls: utils.WatchedChangeBatch[] = [];
            const events = [
                { eventType: 'change' as const, filename: 'a.svg' },
                { eventType: 'change' as const, filename: 'b.svg' },
            ];
            async function* mock() {
                yield events[0]!;
                await setTimeout(40);
                yield events[1]!;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }
            handler.mockImplementationOnce(async (changes: utils.WatchedChangeBatch) => {
                calls.push(changes);
                await firstHandler.promise;
            });
            handler.mockImplementationOnce((changes: utils.WatchedChangeBatch) => {
                calls.push(changes);
            });

            const watcher = utils.setupWatcher(folderPath, ac.signal, handler);
            await setTimeout(100);
            expect(calls).toEqual([[{ path: pathJoin(folderPath, 'a.svg'), kind: 'changed' }]]);

            firstHandler.resolve();
            await watcher;

            expect(calls).toEqual([[{ path: pathJoin(folderPath, 'a.svg'), kind: 'changed' }], [{ path: pathJoin(folderPath, 'b.svg'), kind: 'changed' }]]);
        });

        it('continues flushing later batches after a scheduled handler rejection', async () => {
            const { watch } = fs;
            const calls: utils.WatchedChangeBatch[] = [];
            const events = [
                { eventType: 'change' as const, filename: 'a.svg' },
                { eventType: 'change' as const, filename: 'b.svg' },
            ];
            async function* mock() {
                yield events[0]!;
                await setTimeout(40);
                yield events[1]!;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }
            handler.mockImplementationOnce((changes: utils.WatchedChangeBatch) => {
                calls.push(changes);
                throw new Error('intentional batch failure');
            });
            handler.mockImplementationOnce((changes: utils.WatchedChangeBatch) => {
                calls.push(changes);
            });

            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).toEqual(undefined);
            expect(calls).toEqual([[{ path: pathJoin(folderPath, 'a.svg'), kind: 'changed' }], [{ path: pathJoin(folderPath, 'b.svg'), kind: 'changed' }]]);
        });

        it('swallows a final drain rejection after the watcher closes', async () => {
            const { watch } = fs;
            const event = { eventType: 'change' as const, filename: 'a.svg' };
            async function* mock() {
                yield event;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }
            handler.mockRejectedValueOnce(new Error('intentional drain failure'));

            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).toEqual(undefined);
            expect(handler).toHaveBeenCalledExactlyOnceWith([{ path: pathJoin(folderPath, 'a.svg'), kind: 'changed' }]);
        });

        it('does not propagate watch event classification errors', async () => {
            const { watch } = fs;
            const rejectedEvent = { eventType: 'change' as const, filename: 'unreadable.svg' };
            const validEvent = { eventType: 'change' as const, filename: 'a.svg' };
            async function* mock() {
                yield rejectedEvent;
                yield validEvent;
            }
            if (vi.isMockFunction(watch)) {
                watch.mockReturnValue(mock());
            }
            const handleWatchEvent = vi.fn<typeof utils.handleWatchEvent>().mockRejectedValueOnce(new Error('intentional classification failure'));
            handleWatchEvent.mockImplementation((...args) => utils.handleWatchEvent(...args));

            expect(await utils.setupWatcher(folderPath, ac.signal, handler, handleWatchEvent)).toEqual(undefined);
            expect(handleWatchEvent).toHaveBeenCalledTimes(2);
            expect(handler).toHaveBeenCalledExactlyOnceWith([{ path: pathJoin(folderPath, 'a.svg'), kind: 'changed' }]);
        });
    });

    describe.concurrent('hasFileExtension', () => {
        it.concurrent('should return true for normal file', () => {
            expect(utils.hasFileExtension('example.svg')).toEqual(true);
        });

        it.concurrent('should return true for file with many dots', () => {
            expect(utils.hasFileExtension('example.with.many.dots.in.file.name.svg')).toEqual(true);
        });

        it.concurrent('should return true for file even if absolute route', () => {
            expect(utils.hasFileExtension('/example/from/route.svg')).toEqual(true);
        });

        it.concurrent('should return false for file without any dot', () => {
            expect(utils.hasFileExtension('example')).toEqual(false);
        });

        it.concurrent('should return false for empty string', () => {
            expect(utils.hasFileExtension('')).toEqual(false);
        });

        it.concurrent('should return false for null', () => {
            expect(utils.hasFileExtension(null)).toEqual(false);
        });

        it.concurrent('should return false for undefined', () => {
            expect(utils.hasFileExtension(undefined)).toEqual(false);
        });
    });

    describe.concurrent('ensureDirExistsAndWriteFile', () => {
        it.concurrent('makes a parent directory and writes file', async () => {
            const dir = '/root/example';
            const file = `${dir}/file.css`;
            const content = 'content';
            await utils.ensureDirExistsAndWriteFile(content, file);
            expect(fs.mkdir).toBeCalledWith(dir, { mode: 0o777, recursive: true });
            expect(fs.writeFile).toBeCalledWith(file, content);
        });
    });

    describe.concurrent('getBufferHash', () => {
        const testData: [string, string][] = [
            // [string, sha256 of string]
            ['test data 1', '05e8fdb3598f91bcc3ce41a196e587b4592c8cdfc371c217274bfda2d24b1b4e'],
            ['test data 2', '26637da1bd793f9011a3d304372a9ec44e36cc677d2bbfba32a2f31f912358fe'],
            ['test data 3', 'b2ce6625a947373fe8d578dca152cf152a5bd8aeca805b2d3b1fb4a340e1a123'],
            ['test data 4', '1e2b98ff6439d48d42ae71c0ea44f3c1e03665a34d1c368ac590aec5dadc48eb'],
            ['test data 5', '225b2e6c5664bb388cc40c9abeb289f9569ebc683ed4fdd76fef8421c32369b5'],
        ];

        it.each(testData)('should generate a correct hash for "%s"', (data, hash) => {
            const calculatedHash = utils.getBufferHash(Buffer.from(data));
            expect(calculatedHash).toEqual(hash);
        });
    });
});
