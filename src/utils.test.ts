import * as fs from 'fs/promises';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as utils from './utils';

vi.mock('fs/promises', async () => {
    const fsPromises = await vi.importActual<typeof import('fs/promises')>('fs/promises');
    const access = vi.fn().mockRejectedValueOnce(new Error());
    return { ...fsPromises, access, watch: vi.fn(fsPromises.watch), mkdir: vi.fn(), writeFile: vi.fn() };
});

describe('utils', () => {
    describe('doesFileExist', () => {
        afterEach(() => {
            vi.restoreAllMocks();
        });

        it("return false if file doesn't have read access", async () => {
            expect(await utils.doesFileExist('foo', 'bar')).to.be.false;
            expect(fs.access).toHaveBeenCalledOnce();
        });

        it('return true if file has read access', async () => {
            expect(await utils.doesFileExist('foo', 'bar')).to.be.true;
            expect(fs.access).toHaveBeenCalledOnce();
        });
    });

    describe('handleWatchEvent', () => {
        afterEach(() => {
            vi.restoreAllMocks();
        });

        const validFileName = 'ex.svg';
        const onIconAdded = vi.fn();
        const doesFileExist = vi.fn();

        it("doesn't execute callback for file changes", async () => {
            await utils.handleWatchEvent('', { eventType: 'change', filename: validFileName }, onIconAdded, doesFileExist);
            expect(doesFileExist).not.toHaveBeenCalled();
            expect(onIconAdded).not.toHaveBeenCalled();
        });

        it("doesn't execute callback for non svg files", async () => {
            await utils.handleWatchEvent('', { eventType: 'rename', filename: 'notsvg.png' }, onIconAdded, doesFileExist);
            expect(doesFileExist).not.toHaveBeenCalled();
            expect(onIconAdded).not.toHaveBeenCalled();
        });

        it("doesn't execute callback for non existent files", async () => {
            await utils.handleWatchEvent('', { eventType: 'rename', filename: validFileName }, onIconAdded, doesFileExist);
            expect(doesFileExist).toHaveBeenCalledOnce();
            expect(onIconAdded).not.toHaveBeenCalled();
        });

        it('execute callback for new/renamed file', async () => {
            doesFileExist.mockResolvedValueOnce(true);
            await utils.handleWatchEvent('', { eventType: 'rename', filename: validFileName }, onIconAdded, doesFileExist);
            expect(doesFileExist).toHaveBeenCalledOnce();
            expect(onIconAdded).toHaveBeenCalledOnce();
        });
    });

    describe('setupWatcher', () => {
        const folderPath = './test-folder';
        const handler = vi.fn();
        let ac: AbortController;

        beforeEach(() => {
            ac = new AbortController();
        });

        it('throws error if no such folder', async () => {
            const err = await utils.setupWatcher(folderPath, ac.signal, handler).catch(e => e);
            expect(err).toBeInstanceOf(Error);
            expect(err.message).to.be.eq(`ENOENT: no such file or directory, watch '${folderPath}'`);
        });

        it('handles AbortError without throwing an error', async () => {
            ac.abort();
            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).to.be.undefined;
        });

        it('triggers the handler for files that exist', async () => {
            const { watch, access } = fs;
            const event = { eventType: 'rename', filename: 'ex.svg' };
            async function* mock() {
                yield event;
                vi.isMockFunction(access) && access.mockRejectedValueOnce(new Error());
                yield event;
                yield event;
            }
            vi.isMockFunction(watch) && watch.mockReturnValue(mock());

            expect(await utils.setupWatcher(folderPath, ac.signal, handler)).to.be.undefined;
            expect(handler).toBeCalledTimes(2);
        });
    });

    describe.concurrent('hasFileExtension', () => {
        it.concurrent('should return true for normal file', () => {
            expect(utils.hasFileExtension('example.svg')).to.be.true;
        });

        it.concurrent('should return true for file with many dots', () => {
            expect(utils.hasFileExtension('example.with.many.dots.in.file.name.svg')).to.be.true;
        });

        it.concurrent('should return true for file even if absolute route', () => {
            expect(utils.hasFileExtension('/example/from/route.svg')).to.be.true;
        });

        it.concurrent('should return false for file without any dot', () => {
            expect(utils.hasFileExtension('example')).to.be.false;
        });

        it.concurrent('should return false for empty string', () => {
            expect(utils.hasFileExtension('')).to.be.false;
        });

        it.concurrent('should return false for null', () => {
            expect(utils.hasFileExtension(null)).to.be.false;
        });

        it.concurrent('should return false for undefined', () => {
            expect(utils.hasFileExtension(undefined)).to.be.false;
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

    describe.concurrent('base64ToArrayBuffer', () => {
        const strings = ['vite-svg-2-webfont', 'test-string-1', 'test-string-2'];

        it.each(strings)('should match "%s"', string => {
            const stringAsBase64 = btoa(string);
            const arrayBuffer = utils.base64ToArrayBuffer(stringAsBase64);
            const decodedString = new TextDecoder('utf-8').decode(arrayBuffer);
            expect(decodedString).toEqual(string);
        });
    });
});
