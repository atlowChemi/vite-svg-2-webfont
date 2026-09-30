import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, expect, it } from 'vite-plus/test';
import { generateWebfonts, type GenerateWebfontsFileOptions } from '../index.js';
import * as templates from '../templates.js';

const fixture = join(import.meta.dirname, '../../../crates/webfont-generator/src/svg/fixtures/unit/add.svg');
let dest: string;
beforeEach(async () => {
    dest = await mkdtemp(join(tmpdir(), 'template-parity-'));
});
afterEach(async () => {
    await rm(dest, { recursive: true, force: true });
});

function options(): GenerateWebfontsFileOptions {
    return {
        dest,
        files: [fixture],
        fontName: 'iconfont',
        fontHeight: 1000,
        ligature: false,
        types: ['svg'],
        order: ['svg'],
        codepoints: { add: 0xe001, remove: 0xe002, search: 0xe003 },
        startCodepoint: 0xe001,
        templateOptions: { baseSelector: '.icon', classPrefix: 'icon-' },
        html: true,
        writeFiles: false,
    };
}

it('keeps shipped CSS and HTML templates byte-identical to engine default rendering', async () => {
    const defaults = await generateWebfonts(options());
    const templated = await generateWebfonts({ ...options(), cssTemplate: templates.css, htmlTemplate: templates.html });
    // Template paths participate in cache hashes. Compare rendering with identical URLs.
    const urls = { svg: 'iconfont.svg?parity#iconfont' };
    expect(templated.generateCss(urls)).toBe(defaults.generateCss(urls));
    expect(templated.generateHtml(urls)).toBe(defaults.generateHtml(urls));
});

it('renders shipped CSS with generated URLs, selectors and codepoints', async () => {
    const result = await generateWebfonts({ ...options(), cssTemplate: templates.css, cssFontsUrl: '/assets/fonts', types: ['svg', 'ttf'], order: ['svg', 'ttf'] });
    const css = result.generateCss();
    for (const text of ['@font-face', 'font-family: "iconfont";', 'url("/assets/fonts/iconfont.svg?', 'format("svg")', 'format("truetype")', '.icon-add:before', '\\e001']) {
        expect(css).toContain(text);
    }
});

it('renders shipped HTML with embedded styles and glyph names', async () => {
    const result = await generateWebfonts({ ...options(), cssTemplate: templates.css, htmlTemplate: templates.html, cssFontsUrl: '/assets/fonts' });
    const html = result.generateHtml();
    for (const text of ['<title>iconfont</title>', '<h1>iconfont</h1>', 'preview__icon', '>add<', 'class="icon icon-add"', 'url("iconfont.svg?']) {
        expect(html).toContain(text);
    }
});

it('rebases shipped HTML font URLs relative to the preview destination', async () => {
    const result = await generateWebfonts({
        ...options(),
        dest: join(dest, 'fonts'),
        cssDest: join(dest, 'styles/iconfont.css'),
        htmlDest: join(dest, 'preview/iconfont.html'),
        cssFontsUrl: '/ignored',
        cssTemplate: templates.css,
        htmlTemplate: templates.html,
    });
    expect(result.generateHtml()).toContain('url("../fonts/iconfont.svg?');
});

it('keeps variant template parity, resolved weights, escaping and default-face specificity', async () => {
    const variantOptions = {
        dest,
        fontName: 'weights',
        fontStyle: 'italic',
        writeFiles: false,
        variants: [
            { name: 'light/alt', files: [fixture], weight: 300 },
            { name: 'bold', files: [fixture], weight: 700, default: true },
        ],
    };
    const defaults = await generateWebfonts(variantOptions);
    const templated = await generateWebfonts({ ...variantOptions, cssTemplate: templates.css, htmlTemplate: templates.html });
    const urls = { woff2: 'weights.woff2?parity', woff: 'weights.woff?parity' };
    const css = templated.generateCss(urls);
    expect(css).toBe(defaults.generateCss(urls));
    expect(templated.generateHtml(urls)).toBe(defaults.generateHtml(urls));
    expect(css.match(/@font-face/g)).toHaveLength(2);
    expect(css.match(/weights\.woff2\?/g)).toHaveLength(2);
    for (const text of ['font-weight: 300;', 'font-weight: 700 !important;', 'font-style: italic;', 'font-synthesis: none;', ':is(.icon).icon--light\\/alt:before']) {
        expect(css).toContain(text);
    }
    const baseRule = css.split('.icon:before {')[1].split('}')[0];
    expect(baseRule).toContain('font-weight: 700;');
    expect(baseRule).not.toContain('font-weight: 700 !important;');
});
