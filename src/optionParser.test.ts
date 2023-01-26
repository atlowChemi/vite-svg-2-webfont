import glob from 'glob';
import * as optionParser from './optionParser';
import { describe, it, expect, vi, afterEach, Mock } from 'vitest';
import type { GeneratedFontTypes } from '@vusion/webfonts-generator';

vi.mock('glob', () => ({ default: { sync: vi.fn() } }));

describe('optionParser', () => {
    describe('parseIconTypesOption', () => {
        it('Returns arrays as received', () => {
            const types: GeneratedFontTypes[] = ['eot', 'svg', 'ttf'];
            expect(optionParser.parseIconTypesOption({ types })).to.eq(types);
        });

        it('Transfers string into an array', () => {
            const type = 'eot';
            const val = optionParser.parseIconTypesOption({ types: type });
            expect(Array.isArray(val)).to.be.true;
            expect(val).to.have.lengthOf(1);
            expect(val[0]).to.eq(type);
        });

        it('Returns default types if no types received', () => {
            expect(optionParser.parseIconTypesOption({})).to.have.lengthOf(5);
        });
    });

    describe('parseFiles', () => {
        afterEach(() => {
            vi.restoreAllMocks();
        });

        const { sync } = glob as unknown as { sync: Mock };

        it('defaults to all svg files in context', () => {
            optionParser.parseFiles({ context: '' });
            expect(sync).toHaveBeenCalledOnce();
            expect(sync).toBeCalledWith('*.svg', { cwd: '' });
        });

        it('concatenates the context to the file name', () => {
            const file = 'ex.svg';
            const context = 'prefix';
            sync.mockReturnValueOnce([file]);
            const resp = optionParser.parseFiles({ context });
            expect(sync).toHaveBeenCalledOnce();
            expect(sync).toBeCalledWith('*.svg', { cwd: context });
            expect(resp).to.be.lengthOf(1);
            expect(resp[0]).to.be.eq(`${context}/${file}`);
        });
    });

    describe('parseOptions', () => {
        const context = '';

        it('returns order identical to types', () => {
            const types: GeneratedFontTypes[] = ['ttf', 'woff', 'svg'];
            const res = optionParser.parseOptions({ context, types });
            expect(res.types).to.be.eq(types);
            expect(res.order).to.be.eq(types);
        });

        it('appends a / to dest', () => {
            const res = optionParser.parseOptions({ context, dest: 'dest' });
            expect(res.dest).to.be.eq('dest/');
        });

        it("defaults dest to context's parent artifacts' folder", () => {
            const parent = '/parent/';
            const resDefault = optionParser.parseOptions({ context: `${parent}exIcons` });
            expect(resDefault.dest).to.be.eq(`${parent}artifacts/`);
            const resExplicit = optionParser.parseOptions({ context: `${parent}exIcons`, dest: parent });
            expect(resExplicit.dest).to.be.eq(parent);
        });

        it('defaults font name to icon font', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.fontName).to.be.eq('iconfont');
            const fontName = 'exampleName';
            const resExplicit = optionParser.parseOptions({ context, fontName });
            expect(resExplicit.fontName).to.be.eq(fontName);
        });

        it('defaults font height', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.fontHeight).to.be.eq(1000);
            const fontHeight = 4000;
            const resExplicit = optionParser.parseOptions({ context, fontHeight });
            expect(resExplicit.fontHeight).to.be.eq(fontHeight);
        });

        it('defaults codepoints', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.codepoints).to.deep.eq({});
            const codepoints = { example: 0x1f };
            const resExplicit = optionParser.parseOptions({ context, codepoints });
            expect(resExplicit.codepoints).to.eq(codepoints);
        });

        it('defaults baseSelector', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.templateOptions.baseSelector).to.eq('.icon');
            const baseSelector = '.selector';
            const resExplicit = optionParser.parseOptions({ context, baseSelector });
            expect(resExplicit.templateOptions.baseSelector).to.eq(baseSelector);
        });

        it('defaults classPrefix', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.templateOptions.classPrefix).to.eq('icon-');
            const classPrefix = 'pre-';
            const resExplicit = optionParser.parseOptions({ context, classPrefix });
            expect(resExplicit.templateOptions.classPrefix).to.eq(classPrefix);
        });

        it('sets html based on html or htmlDest', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.html).to.be.false;

            const resExplicitHtml = optionParser.parseOptions({ context, html: true });
            expect(resExplicitHtml.html).to.be.true;

            const resExplicitDest = optionParser.parseOptions({ context, htmlDest: 'example' });
            expect(resExplicitDest.html).to.be.true;

            const resExplicitDestAndHtml = optionParser.parseOptions({ context, html: false, htmlDest: 'example' });
            expect(resExplicitDestAndHtml.html).to.be.false;
        });

        it('sets css based on css or cssDest', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.css).to.be.false;

            const resExplicitCss = optionParser.parseOptions({ context, css: true });
            expect(resExplicitCss.css).to.be.true;

            const resExplicitDest = optionParser.parseOptions({ context, cssDest: 'example' });
            expect(resExplicitDest.css).to.be.true;

            const resExplicitDestAndCss = optionParser.parseOptions({ context, css: false, cssDest: 'example' });
            expect(resExplicitDestAndCss.css).to.be.false;
        });

        it('defaults ligature', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.ligature).to.be.true;
            const resExplicit = optionParser.parseOptions({ context, ligature: false });
            expect(resExplicit.ligature).to.be.false;
        });

        it('defaults writeFiles', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.writeFiles).to.be.false;
            const resExplicit = optionParser.parseOptions({ context, writeFiles: true });
            expect(resExplicit.writeFiles).to.be.true;
        });

        it('defaults formatOptions', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect(resDefault.formatOptions).to.deep.eq({});
            const formatOptions = { svg: {}, woff: {} };
            const resExplicit = optionParser.parseOptions({ context, formatOptions });
            expect(resExplicit.formatOptions).to.eq(formatOptions);
        });

        it('sets cssDest only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssDest' in resDefault).to.be.false;
            const cssDest = '/cssDest';
            const resExplicit = optionParser.parseOptions({ context, cssDest });
            expect(resExplicit.cssDest).to.eq(`${cssDest}/iconfont.css`);
        });

        it('concatenates dest to cssDest', () => {
            const dest = '/root';
            const cssDest = 'cssDest';
            const resExplicit = optionParser.parseOptions({ context, dest, cssDest });
            expect(resExplicit.cssDest).to.eq(`${dest}/${cssDest}/iconfont.css`);
        });

        it('sets cssTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssTemplate' in resDefault).to.be.false;
            const cssTemplate = '/cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, cssTemplate });
            expect(resExplicit.cssTemplate).to.eq(cssTemplate);
        });

        it('concatenates dest to cssTemplate', () => {
            const dest = '/root';
            const cssTemplate = 'cssTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, cssTemplate });
            expect(resExplicit.cssTemplate).to.eq(`${dest}/${cssTemplate}`);
        });

        it('sets cssFontsUrl only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('cssFontsUrl' in resDefault).to.be.false;
            const cssFontsUrl = '/cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, cssFontsUrl });
            expect(resExplicit.cssFontsUrl).to.eq(cssFontsUrl);
        });

        it('concatenates dest to cssFontsUrl', () => {
            const dest = '/root';
            const cssFontsUrl = 'cssFontsUrl';
            const resExplicit = optionParser.parseOptions({ context, dest, cssFontsUrl });
            expect(resExplicit.cssFontsUrl).to.eq(`${dest}/${cssFontsUrl}`);
        });

        it('sets htmlTemplate only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('htmlTemplate' in resDefault).to.be.false;
            const htmlTemplate = '/htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, htmlTemplate });
            expect(resExplicit.htmlTemplate).to.eq(htmlTemplate);
        });

        it('concatenates dest to htmlTemplate', () => {
            const dest = '/root';
            const htmlTemplate = 'htmlTemplate';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlTemplate });
            expect(resExplicit.htmlTemplate).to.eq(`${dest}/${htmlTemplate}`);
        });

        it('sets htmlDest only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('htmlDest' in resDefault).to.be.false;
            const htmlDest = '/htmlDest';
            const resExplicit = optionParser.parseOptions({ context, htmlDest });
            expect(resExplicit.htmlDest).to.eq(`${htmlDest}/iconfont.html`);
        });

        it('concatenates dest to htmlDest', () => {
            const dest = '/root';
            const htmlDest = 'htmlDest';
            const resExplicit = optionParser.parseOptions({ context, dest, htmlDest });
            expect(resExplicit.htmlDest).to.eq(`${dest}/${htmlDest}/iconfont.html`);
        });

        it('sets fixedWidth only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('fixedWidth' in resDefault).to.be.false;
            const resExplicitFalse = optionParser.parseOptions({ context, fixedWidth: false });
            expect(resExplicitFalse.fixedWidth).to.be.false;
            const resExplicitTrue = optionParser.parseOptions({ context, fixedWidth: true });
            expect(resExplicitTrue.fixedWidth).to.be.true;
        });

        it('sets centerHorizontally only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('centerHorizontally' in resDefault).to.be.false;
            const resExplicitFalse = optionParser.parseOptions({ context, centerHorizontally: false });
            expect(resExplicitFalse.centerHorizontally).to.be.false;
            const resExplicitTrue = optionParser.parseOptions({ context, centerHorizontally: true });
            expect(resExplicitTrue.centerHorizontally).to.be.true;
        });

        it('sets normalize only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('normalize' in resDefault).to.be.false;
            const resExplicitFalse = optionParser.parseOptions({ context, normalize: false });
            expect(resExplicitFalse.normalize).to.be.false;
            const resExplicitTrue = optionParser.parseOptions({ context, normalize: true });
            expect(resExplicitTrue.normalize).to.be.true;
        });

        it('sets round only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('round' in resDefault).to.be.false;
            const resExplicitFalsy = optionParser.parseOptions({ context, round: 0 });
            expect(resExplicitFalsy.round).to.eq(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, round: 100 });
            expect(resExplicitTruthy.round).to.eq(100);
        });

        it('sets descent only if defined in options', () => {
            const resDefault = optionParser.parseOptions({ context });
            expect('descent' in resDefault).to.be.false;
            const resExplicitFalsy = optionParser.parseOptions({ context, descent: 0 });
            expect(resExplicitFalsy.descent).to.eq(0);
            const resExplicitTruthy = optionParser.parseOptions({ context, descent: 100 });
            expect(resExplicitTruthy.descent).to.eq(100);
        });
    });
});
