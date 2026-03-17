import * as optionParser from './optionParser';
import type { globSync as GlobSyncFn } from 'glob';
import { describe, it, expect, vi, afterEach, beforeAll } from 'vitest';
import { NoIconsAvailableError, InvalidWriteFilesTypeError } from './errors';
import type { GeneratedFontTypes } from '@vusion/webfonts-generator';

const globSyncMock = vi.hoisted(() => vi.fn<typeof GlobSyncFn>());
vi.mock('glob', () => ({ globSync: globSyncMock }));
const cssContext = () => {
    throw new Error("Shouldn't be called!");
};

describe('optionParser', () => {
    describe.concurrent('parseIconTypesOption', () => {
        it.concurrent('returns arrays as received', () => {
            const types: GeneratedFontTypes[] = ['eot', 'svg', 'ttf'];
            expect(optionParser.parseIconTypesOption({ types })).toBe(types);
        });

        it.concurrent('transfers string into an array', () => {
            const type = 'eot';
            const val = optionParser.parseIconTypesOption({ types: type });
            expect(Array.isArray(val)).toBe(true);
            expect(val).toHaveLength(1);
            expect(val[0]).toBe(type);
        });

        it.concurrent('returns default types if no types received', () => {
            expect(optionParser.parseIconTypesOption({})).toHaveLength(5);
        });
    });

    describe('parseFiles', () => {
        afterEach(() => {
            vi.resetAllMocks();
        });

        it('defaults to all svg files in context', () => {
            try {
                optionParser.parseFiles({ context: '' });
            } catch {
                /* ignore */
            }
            expect(globSyncMock).toHaveBeenCalledOnce();
            expect(globSyncMock).toHaveBeenCalledWith(['*.svg'], { cwd: '' });
        });

        it('concatenates the context to the file name', () => {
            const file = 'ex.svg';
            const context = 'prefix';
            vi.mocked(globSyncMock).mockReturnValueOnce([file]);
            const resp = optionParser.parseFiles({ context });
            expect(globSyncMock).toHaveBeenCalledOnce();
            expect(globSyncMock).toHaveBeenCalledWith(['*.svg'], { cwd: context });
            expect(resp).toHaveLength(1);
            expect(resp[0]).toBe(`${context}/${file}`);
        });

        it('throws if no files found', () => {
            vi.mocked(globSyncMock).mockReturnValueOnce([]);
            let error: unknown;
            try {
                optionParser.parseFiles({ context: '' });
                expect.fail('Should never get to this error!');
            } catch (err) {
                error = err;
            }
            expect(error).toBeInstanceOf(NoIconsAvailableError);
            expect(globSyncMock).toHaveBeenCalledOnce();
            expect(globSyncMock).toHaveBeenCalledWith(['*.svg'], { cwd: '' });
        });
    });

    describe.concurrent('resolveFileDest', () => {
        const globalDest = '/global';
        const fontName = 'fontname';
        const extension = 'css';
        it.concurrent("doesn't concatenate fileDest if not set", () => {
            expect(optionParser.resolveFileDest(globalDest, undefined, fontName, extension)).toBe(`${globalDest}/${fontName}.${extension}`);
        });

        it.concurrent("doesn't concatenate fontName if fileDest has a file extension", () => {
            expect(optionParser.resolveFileDest(globalDest, `file.${extension}`, fontName, extension)).toBe(`${globalDest}/file.${extension}`);
        });

        it.concurrent("concatenates fontName if fileDest doesn't have a file extension", () => {
            expect(optionParser.resolveFileDest(globalDest, 'file', fontName, extension)).toBe(`${globalDest}/file/${fontName}.${extension}`);
        });

        it.concurrent("doesn't concatenate globalDest if fileDest is absolute", () => {
            expect(optionParser.resolveFileDest(globalDest, '/file', fontName, extension)).toBe(`/file/${fontName}.${extension}`);
            expect(optionParser.resolveFileDest(globalDest, `/file.${extension}`, fontName, extension)).toBe(`/file.${extension}`);
        });
    });

    describe.concurrent('buildFileTypeList', () => {
        it.concurrent('returns empty array if generateFiles was undefined', () => {
            expect(optionParser.buildFileTypeList({})).toEqual([]);
            expect(optionParser.buildFileTypeList({ generateFiles: undefined })).toEqual([]);
        });

        it.concurrent('returns empty array if generateFiles was false', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: false })).toEqual([]);
        });

        it.concurrent('returns all options if generateFiles was true', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: true })).toEqual(['html', 'css', 'fonts']);
        });

        it.concurrent('casts values to array', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: 'html' })).toEqual(['html']);
            expect(optionParser.buildFileTypeList({ generateFiles: 'css' })).toEqual(['css']);
            expect(optionParser.buildFileTypeList({ generateFiles: 'fonts' })).toEqual(['fonts']);
        });

        it.concurrent('returns array unchanged', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: ['html'] })).toEqual(['html']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['css'] })).toEqual(['css']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['fonts'] })).toEqual(['fonts']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['html', 'css'] })).toEqual(['html', 'css']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['html', 'fonts'] })).toEqual(['html', 'fonts']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['css', 'fonts'] })).toEqual(['css', 'fonts']);
        });

        it.concurrent('throws an error if received invalid value', () => {
            let error: unknown;
            try {
                // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- explicit for testing invalid value
                optionParser.buildFileTypeList({ generateFiles: 'invalid' as never });
                expect.fail('Should never get to this error!');
            } catch (err) {
                error = err;
            }
            expect(error).toBeInstanceOf(InvalidWriteFilesTypeError);
            try {
                // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- explicit for testing invalid value
                optionParser.buildFileTypeList({ generateFiles: ['invalid'] as never });
                expect.fail('Should never get to this error!');
            } catch (err) {
                error = err;
            }
            expect(error).toBeInstanceOf(InvalidWriteFilesTypeError);
        });
    });

    describe.concurrent('parseGenerateFilesOption', () => {
        describe.concurrent('html', () => {
            it.concurrent('returns false if not set', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: undefined });
                expect(html).toEqual(false);
            });

            it.concurrent('returns false if set to false', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: false });
                expect(html).toEqual(false);
            });

            it.concurrent('returns true if set to true', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: true });
                expect(html).toEqual(true);
            });

            it.concurrent('returns true if value available as string', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: 'html' });
                expect(html).toEqual(true);
            });

            it.concurrent('returns true if value available once in array', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: ['html'] });
                expect(html).toEqual(true);
            });

            it.concurrent('returns true if value available multiple times', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: ['html', 'html'] });
                expect(html).toEqual(true);
            });
        });

        describe.concurrent('css', () => {
            it.concurrent('returns false if not set', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: undefined });
                expect(css).toEqual(false);
            });

            it.concurrent('returns false if set to false', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: false });
                expect(css).toEqual(false);
            });

            it.concurrent('returns true if set to true', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: true });
                expect(css).toEqual(true);
            });

            it.concurrent('returns true if value available as string', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: 'css' });
                expect(css).toEqual(true);
            });

            it.concurrent('returns true if value available once in array', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: ['css'] });
                expect(css).toEqual(true);
            });

            it.concurrent('returns true if value available multiple times', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: ['css', 'css'] });
                expect(css).toEqual(true);
            });
        });

        describe.concurrent('fonts', () => {
            it.concurrent('returns false if not set', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: undefined });
                expect(fonts).toEqual(false);
            });

            it.concurrent('returns false if set to false', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: false });
                expect(fonts).toEqual(false);
            });

            it.concurrent('returns true if set to true', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: true });
                expect(fonts).toEqual(true);
            });

            it.concurrent('returns true if value available as string', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: 'fonts' });
                expect(fonts).toEqual(true);
            });

            it.concurrent('returns true if value available once in array', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: ['fonts'] });
                expect(fonts).toEqual(true);
            });

            it.concurrent('returns true if value available multiple times', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: ['fonts', 'fonts'] });
                expect(fonts).toEqual(true);
            });
        });
    });

    describe.concurrent('parseOptions', () => {
        const context = '';
        beforeAll(() => {
            vi.mocked(globSyncMock).mockReturnValue(['']);
        });

        it.concurrent('returns order identical to types', () => {
            const types: GeneratedFontTypes[] = ['ttf', 'woff', 'svg'];
            const res = optionParser.parseOptions({ context, types });
            expect(res.types).toEqual(types);
            expect(res.order).toEqual(types);
        });

        it.concurrent('appends a / to dest', () => {
            const res = optionParser.parseOptions({ context, dest: 'dest' });
            expect(res.dest).toBe('dest/');
        });

        it.concurrent("defaults dest to context's parent artifacts' folder", () => {
            const parent = '/parent/';
            const resDefault = optionParser.parseOptions({ context: `${parent}exIcons` });
            expect(resDefault.dest).toBe(`${parent}artifacts/`);
            const resExplicit = optionParser.parseOptions({ context: `${parent}exIcons`, dest: parent });
            expect(resExplicit.dest).toBe(parent);
        });

        it.concurrent('defaults font name to icon font', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.fontName).toBe('iconfont');
            const fontName = 'exampleName';
            const resExplicit = optionParser.parseOptions({ context, fontName });
            expect(resExplicit.fontName).toBe(fontName);
        });

        it.concurrent('defaults font height', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.fontHeight).toBe(1000);
            const fontHeight = 4000;
            const resExplicit = optionParser.parseOptions({ context, fontHeight });
            expect(resExplicit.fontHeight).toBe(fontHeight);
        });

        it.concurrent('defaults codepoints', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.codepoints).toEqual({});
            const codepoints = { example: 0x1f };
            const resExplicit = optionParser.parseOptions({ context, codepoints });
            expect(resExplicit.codepoints).toEqual(codepoints);
        });

        it.concurrent('defaults baseSelector', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.templateOptions.baseSelector).toBe('.icon');
            const baseSelector = '.selector';
            const resExplicit = optionParser.parseOptions({ context, baseSelector });
            expect(resExplicit.templateOptions.baseSelector).toBe(baseSelector);
        });

        it.concurrent('defaults classPrefix', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.templateOptions.classPrefix).toBe('icon-');
            const classPrefix = 'pre-';
            const resExplicit = optionParser.parseOptions({ context, classPrefix });
            expect(resExplicit.templateOptions.classPrefix).toBe(classPrefix);
        });

        it.concurrent('sets html based on generateFiles', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.html).toBe(false);

            const resExplicitHtml = optionParser.parseOptions({ context, generateFiles: 'html' });
            expect(resExplicitHtml.html).toBe(true);

            const resExplicitHtmlInArr = optionParser.parseOptions({ context, generateFiles: ['html'] });
            expect(resExplicitHtmlInArr.html).toBe(true);

            const resExplicitFalse = optionParser.parseOptions({ context, generateFiles: false });
            expect(resExplicitFalse.html).toBe(false);

            const resExplicitTrue = optionParser.parseOptions({ context, generateFiles: true });
            expect(resExplicitTrue.html).toBe(true);
        });

        it.concurrent('sets css based on generateFiles', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.css).toBe(false);

            const resExplicitCss = optionParser.parseOptions({ context, generateFiles: 'css' });
            expect(resExplicitCss.css).toBe(true);

            const resExplicitCssInArr = optionParser.parseOptions({ context, generateFiles: ['css'] });
            expect(resExplicitCssInArr.css).toBe(true);

            const resExplicitFalse = optionParser.parseOptions({ context, generateFiles: false });
            expect(resExplicitFalse.css).toBe(false);

            const resExplicitTrue = optionParser.parseOptions({ context, generateFiles: true });
            expect(resExplicitTrue.css).toBe(true);
        });

        it.concurrent('defaults writeFiles', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.writeFiles).toEqual(false);
            const resExplicit = optionParser.parseOptions({ context, generateFiles: true });
            expect(resExplicit.writeFiles).toEqual(true);
            const resExplicitFonts = optionParser.parseOptions({ context, generateFiles: 'fonts' });
            expect(resExplicitFonts.writeFiles).toEqual(true);
            const resExplicitFontsInArr = optionParser.parseOptions({ context, generateFiles: ['fonts'] });
            expect(resExplicitFontsInArr.writeFiles).toEqual(true);
        });

        it.concurrent('defaults ligature', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.ligature).toEqual(true);
            const resExplicit = optionParser.parseOptions({ context, ligature: false });
            expect(resExplicit.ligature).toEqual(false);
        });

        it.concurrent('defaults formatOptions', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.formatOptions).toEqual({});
            const formatOptions = { svg: {}, woff: {} };
            const resExplicit = optionParser.parseOptions({ context, formatOptions });
            expect(resExplicit.formatOptions).toEqual(formatOptions);
        });

        it.concurrent('sets cssDest with default', () => {
            const exampleContext = '/example';
            const resDefault = optionParser.parseOptions({ context: exampleContext });
            expect(resDefault.cssDest).toBe('/artifacts/iconfont.css');
            const cssDest = '/cssDest';
            const resExplicit = optionParser.parseOptions({ context: exampleContext, cssDest });
            expect(resExplicit.cssDest).toBe(`${cssDest}/iconfont.css`);
        });

        it.concurrent('sets htmlDest with default', () => {
            const exampleContext = '/example';
            const resDefault = optionParser.parseOptions({ context: exampleContext });
            expect(resDefault.htmlDest).toBe('/artifacts/iconfont.html');
            const htmlDest = '/htmlDest';
            const resExplicit = optionParser.parseOptions({ context: exampleContext, htmlDest });
            expect(resExplicit.htmlDest).toBe(`${htmlDest}/iconfont.html`);
        });

        it.concurrent('concatenates dest to cssDest', () => {
            const dest = '/root';
            const cssDest = 'cssDest';
            const resExplicit = optionParser.parseOptions({ context, dest, cssDest });
            expect(resExplicit.cssDest).toBe(`${dest}/${cssDest}/iconfont.css`);
        });

        it.concurrent("doesn't concatenate fontName to cssDest, if cssDest is a fileName", () => {
            const dest = '/root';
            const cssDest = 'cssDest.css';
            const resExplicit = optionParser.parseOptions({ context, dest, cssDest });
            expect(resExplicit.cssDest).toBe(`${dest}/${cssDest}`);
        });

        it.concurrent('sets cssTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssTemplate' in resDefault).toEqual(false);
            const cssTemplate = '/cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, cssTemplate });
            expect(resExplicit.cssTemplate).toBe(cssTemplate);
        });

        it.concurrent('sets cssContext only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssContext' in resDefault).toEqual(false);
            const resExplicit = optionParser.parseOptions({ context, cssContext });
            expect(resExplicit.cssContext).toBe(cssContext);
        });

        it.concurrent('concatenates dest to cssTemplate', () => {
            const dest = '/root';
            const cssTemplate = 'cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, cssTemplate });
            expect(resExplicit.cssTemplate).toBe(`${dest}/${cssTemplate}`);
        });

        it.concurrent('sets cssFontsUrl only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssFontsUrl' in resDefault).toEqual(false);
            const cssFontsUrl = '/cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, cssFontsUrl });
            expect(resExplicit.cssFontsUrl).toBe(cssFontsUrl);
        });

        it.concurrent('concatenates dest to cssFontsUrl', () => {
            const dest = '/root';
            const cssFontsUrl = 'cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, dest, cssFontsUrl });
            expect(resExplicit.cssFontsUrl).toBe(`${dest}/${cssFontsUrl}`);
        });

        it.concurrent('sets htmlTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('htmlTemplate' in resDefault).toEqual(false);
            const htmlTemplate = '/htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, htmlTemplate });
            expect(resExplicit.htmlTemplate).toBe(htmlTemplate);
        });

        it.concurrent('concatenates dest to htmlTemplate', () => {
            const dest = '/root';
            const htmlTemplate = 'htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlTemplate });
            expect(resExplicit.htmlTemplate).toBe(`${dest}/${htmlTemplate}`);
        });

        it.concurrent('concatenates dest to htmlDest', () => {
            const dest = '/root';
            const htmlDest = 'htmlDest';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlDest });
            expect(resExplicit.htmlDest).toBe(`${dest}/${htmlDest}/iconfont.html`);
        });

        it.concurrent("doesn't concatenate fontName to htmlDest, if htmlDest is a fileName", () => {
            const dest = '/root';
            const htmlDest = 'htmlDest.ts';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlDest });
            expect(resExplicit.htmlDest).toBe(`${dest}/${htmlDest}`);
        });

        it.concurrent('sets fixedWidth only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('fixedWidth' in resDefault).toEqual(false);
            const resExplicitFalse = optionParser.parseOptions({ context, fixedWidth: false });
            expect(resExplicitFalse.fixedWidth).toEqual(false);
            const resExplicitTrue = optionParser.parseOptions({ context, fixedWidth: true });
            expect(resExplicitTrue.fixedWidth).toEqual(true);
        });

        it.concurrent('sets centerHorizontally only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('centerHorizontally' in resDefault).toEqual(false);
            const resExplicitFalse = optionParser.parseOptions({ context, centerHorizontally: false });
            expect(resExplicitFalse.centerHorizontally).toEqual(false);
            const resExplicitTrue = optionParser.parseOptions({ context, centerHorizontally: true });
            expect(resExplicitTrue.centerHorizontally).toEqual(true);
        });

        it.concurrent('sets normalize only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('normalize' in resDefault).toEqual(false);
            const resExplicitFalse = optionParser.parseOptions({ context, normalize: false });
            expect(resExplicitFalse.normalize).toEqual(false);
            const resExplicitTrue = optionParser.parseOptions({ context, normalize: true });
            expect(resExplicitTrue.normalize).toEqual(true);
        });

        it.concurrent('sets round only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('round' in resDefault).toEqual(false);
            const resExplicitFalsy = optionParser.parseOptions({ context, round: 0 });
            expect(resExplicitFalsy.round).toBe(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, round: 100 });
            expect(resExplicitTruthy.round).toBe(100);
        });

        it.concurrent('sets descent only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('descent' in resDefault).toEqual(false);
            const resExplicitFalsy = optionParser.parseOptions({ context, descent: 0 });
            expect(resExplicitFalsy.descent).toBe(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, descent: 100 });
            expect(resExplicitTruthy.descent).toBe(100);
        });
    });
});
