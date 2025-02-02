import * as optionParser from './optionParser';
import { globSync } from 'glob';
import { describe, it, expect, vi, afterEach, beforeAll } from 'vitest';
import { NoIconsAvailableError, InvalidWriteFilesTypeError } from './errors';
import type { GeneratedFontTypes } from '@vusion/webfonts-generator';

vi.mock('glob', () => ({ globSync: vi.fn() }));

describe('optionParser', () => {
    describe.concurrent('parseIconTypesOption', () => {
        it.concurrent('returns arrays as received', () => {
            const types: GeneratedFontTypes[] = ['eot', 'svg', 'ttf'];
            expect(optionParser.parseIconTypesOption({ types })).to.eq(types);
        });

        it.concurrent('transfers string into an array', () => {
            const type = 'eot';
            const val = optionParser.parseIconTypesOption({ types: type });
            expect(Array.isArray(val)).toEqual(true);
            expect(val).to.have.lengthOf(1);
            expect(val[0]).to.eq(type);
        });

        it.concurrent('returns default types if no types received', () => {
            expect(optionParser.parseIconTypesOption({})).to.have.lengthOf(5);
        });
    });

    describe('parseFiles', () => {
        afterEach(() => {
            vi.restoreAllMocks();
        });

        it('defaults to all svg files in context', () => {
            try {
                optionParser.parseFiles({ context: '' });
            } catch {
                /* ignore */
            }
            expect(globSync).toHaveBeenCalledOnce();
            expect(globSync).toBeCalledWith(['*.svg'], { cwd: '' });
        });

        it('concatenates the context to the file name', () => {
            const file = 'ex.svg';
            const context = 'prefix';
            vi.mocked(globSync).mockReturnValueOnce([file]);
            const resp = optionParser.parseFiles({ context });
            expect(globSync).toHaveBeenCalledOnce();
            expect(globSync).toBeCalledWith(['*.svg'], { cwd: context });
            expect(resp).to.be.lengthOf(1);
            expect(resp[0]).to.be.eq(`${context}/${file}`);
        });

        it('throws if no files found', () => {
            vi.mocked(globSync).mockReturnValueOnce([]);
            try {
                optionParser.parseFiles({ context: '' });
                throw new Error('Should never get to this error!');
            } catch (err) {
                expect(err).to.be.instanceOf(NoIconsAvailableError);
            }
            expect(globSync).toHaveBeenCalledOnce();
            expect(globSync).toBeCalledWith(['*.svg'], { cwd: '' });
        });
    });

    describe.concurrent('resolveFileDest', () => {
        const globalDest = '/global';
        const fontName = 'fontname';
        const extension = 'css';
        it.concurrent("doesn't concatenate fileDest if not set", () => {
            expect(optionParser.resolveFileDest(globalDest, undefined, fontName, extension)).to.eq(`${globalDest}/${fontName}.${extension}`);
        });

        it.concurrent("doesn't concatenate fontName if fileDest has a file extension", () => {
            expect(optionParser.resolveFileDest(globalDest, `file.${extension}`, fontName, extension)).to.eq(`${globalDest}/file.${extension}`);
        });

        it.concurrent("concatenates fontName if fileDest doesn't have a file extension", () => {
            expect(optionParser.resolveFileDest(globalDest, 'file', fontName, extension)).to.eq(`${globalDest}/file/${fontName}.${extension}`);
        });

        it.concurrent("doesn't concatenate globalDest if fileDest is absolute", () => {
            expect(optionParser.resolveFileDest(globalDest, '/file', fontName, extension)).to.eq(`/file/${fontName}.${extension}`);
            expect(optionParser.resolveFileDest(globalDest, `/file.${extension}`, fontName, extension)).to.eq(`/file.${extension}`);
        });
    });

    describe.concurrent('buildFileTypeList', () => {
        it.concurrent('returns empty array if generateFiles was undefined', () => {
            expect(optionParser.buildFileTypeList({})).to.deep.eq([]);
            expect(optionParser.buildFileTypeList({ generateFiles: undefined })).to.deep.eq([]);
        });

        it.concurrent('returns empty array if generateFiles was false', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: false })).to.deep.eq([]);
        });

        it.concurrent('returns all options if generateFiles was true', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: true })).to.deep.eq(['html', 'css', 'fonts']);
        });

        it.concurrent('casts values to array', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: 'html' })).to.deep.eq(['html']);
            expect(optionParser.buildFileTypeList({ generateFiles: 'css' })).to.deep.eq(['css']);
            expect(optionParser.buildFileTypeList({ generateFiles: 'fonts' })).to.deep.eq(['fonts']);
        });

        it.concurrent('returns array unchanged', () => {
            expect(optionParser.buildFileTypeList({ generateFiles: ['html'] })).to.deep.eq(['html']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['css'] })).to.deep.eq(['css']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['fonts'] })).to.deep.eq(['fonts']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['html', 'css'] })).to.deep.eq(['html', 'css']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['html', 'fonts'] })).to.deep.eq(['html', 'fonts']);
            expect(optionParser.buildFileTypeList({ generateFiles: ['css', 'fonts'] })).to.deep.eq(['css', 'fonts']);
        });

        it.concurrent('throws an error if received invalid value', () => {
            try {
                optionParser.buildFileTypeList({ generateFiles: 'invalid' as never });
                throw new Error('Should never get to this error!');
            } catch (err) {
                expect(err).to.be.instanceOf(InvalidWriteFilesTypeError);
            }
            try {
                optionParser.buildFileTypeList({ generateFiles: ['invalid'] as never });
                throw new Error('Should never get to this error!');
            } catch (err) {
                expect(err).to.be.instanceOf(InvalidWriteFilesTypeError);
            }
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
            vi.mocked(globSync).mockReturnValue(['']);
        });

        it.concurrent('returns order identical to types', () => {
            const types: GeneratedFontTypes[] = ['ttf', 'woff', 'svg'];
            const res = optionParser.parseOptions({ context, types });
            expect(res.types).to.be.eq(types);
            expect(res.order).to.be.eq(types);
        });

        it.concurrent('appends a / to dest', () => {
            const res = optionParser.parseOptions({ context, dest: 'dest' });
            expect(res.dest).to.be.eq('dest/');
        });

        it.concurrent("defaults dest to context's parent artifacts' folder", () => {
            const parent = '/parent/';
            const resDefault = optionParser.parseOptions({ context: `${parent}exIcons` });
            expect(resDefault.dest).to.be.eq(`${parent}artifacts/`);
            const resExplicit = optionParser.parseOptions({ context: `${parent}exIcons`, dest: parent });
            expect(resExplicit.dest).to.be.eq(parent);
        });

        it.concurrent('defaults font name to icon font', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.fontName).to.be.eq('iconfont');
            const fontName = 'exampleName';
            const resExplicit = optionParser.parseOptions({ context, fontName });
            expect(resExplicit.fontName).to.be.eq(fontName);
        });

        it.concurrent('defaults font height', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.fontHeight).to.be.eq(1000);
            const fontHeight = 4000;
            const resExplicit = optionParser.parseOptions({ context, fontHeight });
            expect(resExplicit.fontHeight).to.be.eq(fontHeight);
        });

        it.concurrent('defaults codepoints', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.codepoints).to.deep.eq({});
            const codepoints = { example: 0x1f };
            const resExplicit = optionParser.parseOptions({ context, codepoints });
            expect(resExplicit.codepoints).to.eq(codepoints);
        });

        it.concurrent('defaults baseSelector', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.templateOptions.baseSelector).to.eq('.icon');
            const baseSelector = '.selector';
            const resExplicit = optionParser.parseOptions({ context, baseSelector });
            expect(resExplicit.templateOptions.baseSelector).to.eq(baseSelector);
        });

        it.concurrent('defaults classPrefix', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.templateOptions.classPrefix).to.eq('icon-');
            const classPrefix = 'pre-';
            const resExplicit = optionParser.parseOptions({ context, classPrefix });
            expect(resExplicit.templateOptions.classPrefix).to.eq(classPrefix);
        });

        it.concurrent('sets html based on generateFiles', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.html).toEqual(false);

            const resExplicitHtml = optionParser.parseOptions({ context, generateFiles: 'html' });
            expect(resExplicitHtml.html).toEqual(true);

            const resExplicitHtmlInArr = optionParser.parseOptions({ context, generateFiles: ['html'] });
            expect(resExplicitHtmlInArr.html).toEqual(true);

            const resExplicitFalse = optionParser.parseOptions({ context, generateFiles: false });
            expect(resExplicitFalse.html).toEqual(false);

            const resExplicitTrue = optionParser.parseOptions({ context, generateFiles: true });
            expect(resExplicitTrue.html).toEqual(true);
        });

        it.concurrent('sets css based on generateFiles', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.css).toEqual(false);

            const resExplicitCss = optionParser.parseOptions({ context, generateFiles: 'css' });
            expect(resExplicitCss.css).toEqual(true);

            const resExplicitCssInArr = optionParser.parseOptions({ context, generateFiles: ['css'] });
            expect(resExplicitCssInArr.css).toEqual(true);

            const resExplicitFalse = optionParser.parseOptions({ context, generateFiles: false });
            expect(resExplicitFalse.css).toEqual(false);

            const resExplicitTrue = optionParser.parseOptions({ context, generateFiles: true });
            expect(resExplicitTrue.css).toEqual(true);
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
            expect(resDefault.formatOptions).to.deep.eq({});
            const formatOptions = { svg: {}, woff: {} };
            const resExplicit = optionParser.parseOptions({ context, formatOptions });
            expect(resExplicit.formatOptions).to.eq(formatOptions);
        });

        it.concurrent('sets cssDest with default', () => {
            const context = '/example';
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.cssDest).to.eq('/artifacts/iconfont.css');
            const cssDest = '/cssDest';
            const resExplicit = optionParser.parseOptions({ context, cssDest });
            expect(resExplicit.cssDest).to.eq(`${cssDest}/iconfont.css`);
        });

        it.concurrent('sets htmlDest with default', () => {
            const context = '/example';
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.htmlDest).to.eq('/artifacts/iconfont.html');
            const htmlDest = '/htmlDest';
            const resExplicit = optionParser.parseOptions({ context, htmlDest });
            expect(resExplicit.htmlDest).to.eq(`${htmlDest}/iconfont.html`);
        });

        it.concurrent('concatenates dest to cssDest', () => {
            const dest = '/root';
            const cssDest = 'cssDest';
            const resExplicit = optionParser.parseOptions({ context, dest, cssDest });
            expect(resExplicit.cssDest).to.eq(`${dest}/${cssDest}/iconfont.css`);
        });

        it.concurrent("doesn't concatenate fontName to cssDest, if cssDest is a fileName", () => {
            const dest = '/root';
            const cssDest = 'cssDest.css';
            const resExplicit = optionParser.parseOptions({ context, dest, cssDest });
            expect(resExplicit.cssDest).to.eq(`${dest}/${cssDest}`);
        });

        it.concurrent('sets cssTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssTemplate' in resDefault).toEqual(false);
            const cssTemplate = '/cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, cssTemplate });
            expect(resExplicit.cssTemplate).to.eq(cssTemplate);
        });

        it.concurrent('sets cssContext only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssContext' in resDefault).toEqual(false);
            const cssContext = () => {
                throw new Error("Shouldn't be called!");
            };
            const resExplicit = optionParser.parseOptions({ context, cssContext });
            expect(resExplicit.cssContext).to.eq(cssContext);
        });

        it.concurrent('concatenates dest to cssTemplate', () => {
            const dest = '/root';
            const cssTemplate = 'cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, cssTemplate });
            expect(resExplicit.cssTemplate).to.eq(`${dest}/${cssTemplate}`);
        });

        it.concurrent('sets cssFontsUrl only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssFontsUrl' in resDefault).toEqual(false);
            const cssFontsUrl = '/cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, cssFontsUrl });
            expect(resExplicit.cssFontsUrl).to.eq(cssFontsUrl);
        });

        it.concurrent('concatenates dest to cssFontsUrl', () => {
            const dest = '/root';
            const cssFontsUrl = 'cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, dest, cssFontsUrl });
            expect(resExplicit.cssFontsUrl).to.eq(`${dest}/${cssFontsUrl}`);
        });

        it.concurrent('sets htmlTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('htmlTemplate' in resDefault).toEqual(false);
            const htmlTemplate = '/htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, htmlTemplate });
            expect(resExplicit.htmlTemplate).to.eq(htmlTemplate);
        });

        it.concurrent('concatenates dest to htmlTemplate', () => {
            const dest = '/root';
            const htmlTemplate = 'htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlTemplate });
            expect(resExplicit.htmlTemplate).to.eq(`${dest}/${htmlTemplate}`);
        });

        it.concurrent('concatenates dest to htmlDest', () => {
            const dest = '/root';
            const htmlDest = 'htmlDest';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlDest });
            expect(resExplicit.htmlDest).to.eq(`${dest}/${htmlDest}/iconfont.html`);
        });

        it.concurrent("doesn't concatenate fontName to htmlDest, if htmlDest is a fileName", () => {
            const dest = '/root';
            const htmlDest = 'htmlDest.ts';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlDest });
            expect(resExplicit.htmlDest).to.eq(`${dest}/${htmlDest}`);
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
            expect(resExplicitFalsy.round).to.eq(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, round: 100 });
            expect(resExplicitTruthy.round).to.eq(100);
        });

        it.concurrent('sets descent only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('descent' in resDefault).toEqual(false);
            const resExplicitFalsy = optionParser.parseOptions({ context, descent: 0 });
            expect(resExplicitFalsy.descent).to.eq(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, descent: 100 });
            expect(resExplicitTruthy.descent).to.eq(100);
        });
    });
});
