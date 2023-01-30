import glob from 'glob';
import * as optionParser from './optionParser';
import { describe, it, expect, vi, afterEach, Mock } from 'vitest';
import { NoIconsAvailableError, InvalidWriteFilesTypeError } from './errors';
import type { GeneratedFontTypes } from '@vusion/webfonts-generator';

vi.mock('glob', () => ({ default: { sync: vi.fn() } }));

describe('optionParser', () => {
    describe.concurrent('parseIconTypesOption', () => {
        it.concurrent('returns arrays as received', () => {
            const types: GeneratedFontTypes[] = ['eot', 'svg', 'ttf'];
            expect(optionParser.parseIconTypesOption({ types })).to.eq(types);
        });

        it.concurrent('transfers string into an array', () => {
            const type = 'eot';
            const val = optionParser.parseIconTypesOption({ types: type });
            expect(Array.isArray(val)).to.be.true;
            expect(val).to.have.lengthOf(1);
            expect(val[0]).to.eq(type);
        });

        it.concurrent('returns default types if no types received', () => {
            expect(optionParser.parseIconTypesOption({})).to.have.lengthOf(5);
        });
    });

    describe.concurrent('parseFiles', () => {
        afterEach(() => {
            vi.restoreAllMocks();
        });

        const { sync } = glob as unknown as { sync: Mock };

        it.concurrent('defaults to all svg files in context', () => {
            optionParser.parseFiles({ context: '' });
            expect(sync).toHaveBeenCalledOnce();
            expect(sync).toBeCalledWith('*.svg', { cwd: '' });
        });

        it.concurrent('concatenates the context to the file name', () => {
            const file = 'ex.svg';
            const context = 'prefix';
            sync.mockReturnValueOnce([file]);
            const resp = optionParser.parseFiles({ context });
            expect(sync).toHaveBeenCalledOnce();
            expect(sync).toBeCalledWith('*.svg', { cwd: context });
            expect(resp).to.be.lengthOf(1);
            expect(resp[0]).to.be.eq(`${context}/${file}`);
        });

        it.concurrent('throws if no files found', async () => {
            sync.mockReturnValueOnce([]);
            try {
                optionParser.parseFiles({ context: '' });
                throw new Error('Should never get to this error!');
            } catch (err) {
                expect(err).to.be.instanceOf(NoIconsAvailableError);
            }
            expect(sync).toHaveBeenCalledOnce();
            expect(sync).toBeCalledWith('*.svg', { cwd: '' });
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
                expect(html).to.be.false;
            });

            it.concurrent('returns false if set to false', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: false });
                expect(html).to.be.false;
            });

            it.concurrent('returns true if set to true', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: true });
                expect(html).to.be.true;
            });

            it.concurrent('returns true if value available as string', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: 'html' });
                expect(html).to.be.true;
            });

            it.concurrent('returns true if value available once in array', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: ['html'] });
                expect(html).to.be.true;
            });

            it.concurrent('returns true if value available multiple times', () => {
                const { html } = optionParser.parseGenerateFilesOption({ generateFiles: ['html', 'html'] });
                expect(html).to.be.true;
            });
        });

        describe.concurrent('css', () => {
            it.concurrent('returns false if not set', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: undefined });
                expect(css).to.be.false;
            });

            it.concurrent('returns false if set to false', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: false });
                expect(css).to.be.false;
            });

            it.concurrent('returns true if set to true', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: true });
                expect(css).to.be.true;
            });

            it.concurrent('returns true if value available as string', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: 'css' });
                expect(css).to.be.true;
            });

            it.concurrent('returns true if value available once in array', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: ['css'] });
                expect(css).to.be.true;
            });

            it.concurrent('returns true if value available multiple times', () => {
                const { css } = optionParser.parseGenerateFilesOption({ generateFiles: ['css', 'css'] });
                expect(css).to.be.true;
            });
        });

        describe.concurrent('fonts', () => {
            it.concurrent('returns false if not set', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: undefined });
                expect(fonts).to.be.false;
            });

            it.concurrent('returns false if set to false', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: false });
                expect(fonts).to.be.false;
            });

            it.concurrent('returns true if set to true', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: true });
                expect(fonts).to.be.true;
            });

            it.concurrent('returns true if value available as string', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: 'fonts' });
                expect(fonts).to.be.true;
            });

            it.concurrent('returns true if value available once in array', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: ['fonts'] });
                expect(fonts).to.be.true;
            });

            it.concurrent('returns true if value available multiple times', () => {
                const { fonts } = optionParser.parseGenerateFilesOption({ generateFiles: ['fonts', 'fonts'] });
                expect(fonts).to.be.true;
            });
        });
    });

    describe.concurrent('parseOptions', () => {
        const context = '';

        it.concurrent('returns processedOptions and generateFilesOptions', () => {
            const res = optionParser.parseOptions({ context });
            expect(res).to.have.property('processedOptions');
            expect(res).to.have.property('generateFilesOptions');
            expect(res.generateFilesOptions.css).to.be.toBeTypeOf('boolean');
            expect(res.generateFilesOptions.html).to.be.toBeTypeOf('boolean');
            expect(res.generateFilesOptions.fonts).to.be.toBeTypeOf('boolean');
        });

        it.concurrent('returns order identical to types', () => {
            const types: GeneratedFontTypes[] = ['ttf', 'woff', 'svg'];
            const res = optionParser.parseOptions({ context, types }).processedOptions;
            expect(res.types).to.be.eq(types);
            expect(res.order).to.be.eq(types);
        });

        it.concurrent('appends a / to dest', () => {
            const res = optionParser.parseOptions({ context, dest: 'dest' }).processedOptions;
            expect(res.dest).to.be.eq('dest/');
        });

        it.concurrent("defaults dest to context's parent artifacts' folder", () => {
            const parent = '/parent/';
            const resDefault = optionParser.parseOptions({ context: `${parent}exIcons` }).processedOptions;
            expect(resDefault.dest).to.be.eq(`${parent}artifacts/`);
            const resExplicit = optionParser.parseOptions({ context: `${parent}exIcons`, dest: parent }).processedOptions;
            expect(resExplicit.dest).to.be.eq(parent);
        });

        it.concurrent('defaults font name to icon font', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.fontName).to.be.eq('iconfont');
            const fontName = 'exampleName';
            const resExplicit = optionParser.parseOptions({ context, fontName }).processedOptions;
            expect(resExplicit.fontName).to.be.eq(fontName);
        });

        it.concurrent('defaults font height', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.fontHeight).to.be.eq(1000);
            const fontHeight = 4000;
            const resExplicit = optionParser.parseOptions({ context, fontHeight }).processedOptions;
            expect(resExplicit.fontHeight).to.be.eq(fontHeight);
        });

        it.concurrent('defaults codepoints', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.codepoints).to.deep.eq({});
            const codepoints = { example: 0x1f };
            const resExplicit = optionParser.parseOptions({ context, codepoints }).processedOptions;
            expect(resExplicit.codepoints).to.eq(codepoints);
        });

        it.concurrent('defaults baseSelector', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.templateOptions.baseSelector).to.eq('.icon');
            const baseSelector = '.selector';
            const resExplicit = optionParser.parseOptions({ context, baseSelector }).processedOptions;
            expect(resExplicit.templateOptions.baseSelector).to.eq(baseSelector);
        });

        it.concurrent('defaults classPrefix', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.templateOptions.classPrefix).to.eq('icon-');
            const classPrefix = 'pre-';
            const resExplicit = optionParser.parseOptions({ context, classPrefix }).processedOptions;
            expect(resExplicit.templateOptions.classPrefix).to.eq(classPrefix);
        });

        it.concurrent('sets html based on generateFiles', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.html).to.be.false;

            const resExplicitHtml = optionParser.parseOptions({ context, generateFiles: 'html' }).processedOptions;
            expect(resExplicitHtml.html).to.be.true;

            const resExplicitHtmlInArr = optionParser.parseOptions({ context, generateFiles: ['html'] }).processedOptions;
            expect(resExplicitHtmlInArr.html).to.be.true;

            const resExplicitFalse = optionParser.parseOptions({ context, generateFiles: false }).processedOptions;
            expect(resExplicitFalse.html).to.be.false;

            const resExplicitTrue = optionParser.parseOptions({ context, generateFiles: true }).processedOptions;
            expect(resExplicitTrue.html).to.be.true;
        });

        it.concurrent('sets css based on generateFiles', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.css).to.be.false;

            const resExplicitCss = optionParser.parseOptions({ context, generateFiles: 'css' }).processedOptions;
            expect(resExplicitCss.css).to.be.true;

            const resExplicitCssInArr = optionParser.parseOptions({ context, generateFiles: ['css'] }).processedOptions;
            expect(resExplicitCssInArr.css).to.be.true;

            const resExplicitFalse = optionParser.parseOptions({ context, generateFiles: false }).processedOptions;
            expect(resExplicitFalse.css).to.be.false;

            const resExplicitTrue = optionParser.parseOptions({ context, generateFiles: true }).processedOptions;
            expect(resExplicitTrue.css).to.be.true;
        });

        it.concurrent('defaults writeFiles', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.writeFiles).to.be.false;
            const resExplicit = optionParser.parseOptions({ context, generateFiles: true }).processedOptions;
            expect(resExplicit.writeFiles).to.be.true;
            const resExplicitFonts = optionParser.parseOptions({ context, generateFiles: 'fonts' }).processedOptions;
            expect(resExplicitFonts.writeFiles).to.be.true;
            const resExplicitFontsInArr = optionParser.parseOptions({ context, generateFiles: ['fonts'] }).processedOptions;
            expect(resExplicitFontsInArr.writeFiles).to.be.true;
        });

        it.concurrent('defaults ligature', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.ligature).to.be.true;
            const resExplicit = optionParser.parseOptions({ context, ligature: false }).processedOptions;
            expect(resExplicit.ligature).to.be.false;
        });

        it.concurrent('defaults formatOptions', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.formatOptions).to.deep.eq({});
            const formatOptions = { svg: {}, woff: {} };
            const resExplicit = optionParser.parseOptions({ context, formatOptions }).processedOptions;
            expect(resExplicit.formatOptions).to.eq(formatOptions);
        });

        it.concurrent('sets cssDest with default', () => {
            const context = '/example';
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.cssDest).to.eq('/artifacts/iconfont.css');
            const cssDest = '/cssDest';
            const resExplicit = optionParser.parseOptions({ context, cssDest }).processedOptions;
            expect(resExplicit.cssDest).to.eq(`${cssDest}/iconfont.css`);
        });
        
        it.concurrent('sets htmlDest with default', () => {
            const context = '/example';
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect(resDefault.htmlDest).to.eq('/artifacts/iconfont.html');
            const htmlDest = '/htmlDest';
            const resExplicit = optionParser.parseOptions({ context, htmlDest }).processedOptions;
            expect(resExplicit.htmlDest).to.eq(`${htmlDest}/iconfont.html`);
        });

        it.concurrent('concatenates dest to cssDest', () => {
            const dest = '/root';
            const cssDest = 'cssDest';
            const resExplicit = optionParser.parseOptions({ context, dest, cssDest }).processedOptions;
            expect(resExplicit.cssDest).to.eq(`${dest}/${cssDest}/iconfont.css`);
        });

        it.concurrent("doesn't concatenate fontName to cssDest, if cssDest is a fileName", () => {
            const dest = '/root';
            const cssDest = 'cssDest.css';
            const resExplicit = optionParser.parseOptions({ context, dest, cssDest }).processedOptions;
            expect(resExplicit.cssDest).to.eq(`${dest}/${cssDest}`);
        });

        it.concurrent('sets cssTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('cssTemplate' in resDefault).to.be.false;
            const cssTemplate = '/cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, cssTemplate }).processedOptions;
            expect(resExplicit.cssTemplate).to.eq(cssTemplate);
        });

        it.concurrent('concatenates dest to cssTemplate', () => {
            const dest = '/root';
            const cssTemplate = 'cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, cssTemplate }).processedOptions;
            expect(resExplicit.cssTemplate).to.eq(`${dest}/${cssTemplate}`);
        });

        it.concurrent('sets cssFontsUrl only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('cssFontsUrl' in resDefault).to.be.false;
            const cssFontsUrl = '/cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, cssFontsUrl }).processedOptions;
            expect(resExplicit.cssFontsUrl).to.eq(cssFontsUrl);
        });

        it.concurrent('concatenates dest to cssFontsUrl', () => {
            const dest = '/root';
            const cssFontsUrl = 'cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, dest, cssFontsUrl }).processedOptions;
            expect(resExplicit.cssFontsUrl).to.eq(`${dest}/${cssFontsUrl}`);
        });

        it.concurrent('sets htmlTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('htmlTemplate' in resDefault).to.be.false;
            const htmlTemplate = '/htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, htmlTemplate }).processedOptions;
            expect(resExplicit.htmlTemplate).to.eq(htmlTemplate);
        });

        it.concurrent('concatenates dest to htmlTemplate', () => {
            const dest = '/root';
            const htmlTemplate = 'htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlTemplate }).processedOptions;
            expect(resExplicit.htmlTemplate).to.eq(`${dest}/${htmlTemplate}`);
        });

        it.concurrent('concatenates dest to htmlDest', () => {
            const dest = '/root';
            const htmlDest = 'htmlDest';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlDest }).processedOptions;
            expect(resExplicit.htmlDest).to.eq(`${dest}/${htmlDest}/iconfont.html`);
        });

        it.concurrent("doesn't concatenate fontName to htmlDest, if htmlDest is a fileName", () => {
            const dest = '/root';
            const htmlDest = 'htmlDest.ts';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlDest }).processedOptions;
            expect(resExplicit.htmlDest).to.eq(`${dest}/${htmlDest}`);
        });

        it.concurrent('sets fixedWidth only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('fixedWidth' in resDefault).to.be.false;
            const resExplicitFalse = optionParser.parseOptions({ context, fixedWidth: false }).processedOptions;
            expect(resExplicitFalse.fixedWidth).to.be.false;
            const resExplicitTrue = optionParser.parseOptions({ context, fixedWidth: true }).processedOptions;
            expect(resExplicitTrue.fixedWidth).to.be.true;
        });

        it.concurrent('sets centerHorizontally only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('centerHorizontally' in resDefault).to.be.false;
            const resExplicitFalse = optionParser.parseOptions({ context, centerHorizontally: false }).processedOptions;
            expect(resExplicitFalse.centerHorizontally).to.be.false;
            const resExplicitTrue = optionParser.parseOptions({ context, centerHorizontally: true }).processedOptions;
            expect(resExplicitTrue.centerHorizontally).to.be.true;
        });

        it.concurrent('sets normalize only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('normalize' in resDefault).to.be.false;
            const resExplicitFalse = optionParser.parseOptions({ context, normalize: false }).processedOptions;
            expect(resExplicitFalse.normalize).to.be.false;
            const resExplicitTrue = optionParser.parseOptions({ context, normalize: true }).processedOptions;
            expect(resExplicitTrue.normalize).to.be.true;
        });

        it.concurrent('sets round only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('round' in resDefault).to.be.false;
            const resExplicitFalsy = optionParser.parseOptions({ context, round: 0 }).processedOptions;
            expect(resExplicitFalsy.round).to.eq(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, round: 100 }).processedOptions;
            expect(resExplicitTruthy.round).to.eq(100);
        });

        it.concurrent('sets descent only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context }).processedOptions;
            expect('descent' in resDefault).to.be.false;
            const resExplicitFalsy = optionParser.parseOptions({ context, descent: 0 }).processedOptions;
            expect(resExplicitFalsy.descent).to.eq(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, descent: 100 }).processedOptions;
            expect(resExplicitTruthy.descent).to.eq(100);
        });
    });
});
