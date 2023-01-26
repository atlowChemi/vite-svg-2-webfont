import glob from 'glob';
import { resolve } from 'path';
import type { WebfontsGeneratorOptions, GeneratedFontTypes } from '@vusion/webfonts-generator';

const { sync } = glob;

export interface IconPluginOptions<T extends GeneratedFontTypes = GeneratedFontTypes> {
    /** Context directory in which the SVG files will be read from */
    context: string;
    /**
     * Name of font and base name of font files.
     * @default 'iconfont'
     */
    fontName?: string;
    /**
     * Directory for generated font files.
     * @default path.resolve(options.context, '..', 'artifacts')
     */
    dest?: string;
    /**
     * Whether to generate CSS file.
     * @default false
     */
    css?: boolean;
    /**
     * Whether to generate HTML preview.
     * @default false
     */
    html?: boolean;
    /**
     * Enable or disable ligature function.
     * @default true
     */
    ligature?: boolean;
    /**
     * Normalize icons by scaling them to the height of the highest icon.
     * @default false
     */
    normalize?: boolean;
    /**
     * Setup SVG path rounding.
     * @default 10e12
     */
    round?: number;
    /**
     * The font descent. It is useful to fix the font baseline yourself.
     * @default 0
     */
    descent?: number;
    /**
     * Creates a monospace font of the width of the largest input icon.
     * @default false
     */
    fixedWidth?: boolean;
    /** Calculate the bounds of a glyph and center it horizontally. */
    centerHorizontally?: boolean;
    /**
     * Path for generated CSS file.
     * @default path.join(options.dest, options.fontName + '.css')
     */
    cssDest?: string;
    /**
     * Path of custom CSS template. Generator uses handlebars templates.
     * Template receives options from options.templateOptions along with the following options:
     * - fontName
     * - src `string` – Value of the `src` property for `@font-face`.
     * - codepoints `object` – Codepoints of icons in hex format.
     *
     * Paths of default templates are stored in the `webfontsGenerator.templates` object.
     * - `webfontsGenerator.templates.css` – Default CSS template path. It generates classes with names based on values from `options.templateOptions`.
     * - `webfontsGenerator.templates.scss` – Default SCSS template path. It generates mixin `webfont-icon` to add icon styles. It is safe to use multiple generated files with mixins together.
     */
    cssTemplate?: string;
    /**
     * Fonts path used in CSS file.
     * @default options.cssDest
     */
    cssFontsUrl?: string;
    /**
     * Path for generated HTML file.
     * @default path.join(options.dest, options.fontName + '.html')
     */
    htmlDest?: string;
    /**
     * HTML template path. Generator uses handlebars templates.
     *
     * Template receives options from `options.templateOptions` along with the following options:
     * - fontName
     * - styles `string` – Styles generated with default CSS template. (`cssFontsPath` is changed to relative path from `htmlDest` to `dest`)
     * - names `string[]` – Names of icons.
     */
    htmlTemplate?: string;
    /**
     * It is possible to not create files and get generated fonts in object to write them to files later.
     *
     * Also results object will have function generateCss([urls]) where urls is an object with future fonts urls.
     * @default false
     */
    writeFiles?: boolean;
    /**
     * Specific per format arbitrary options to pass to the generator.
     *
     * Format and matching generator:
     * - svg - [svgicons2svgfont](https://github.com/nfroidure/svgicons2svgfont).
     * - ttf - [svg2ttf](https://github.com/fontello/svg2ttf).
     * - woff2 - [ttf2woff2](https://github.com/nfroidure/ttf2woff2).
     * - woff - [ttf2woff](https://github.com/fontello/ttf2woff).
     * - eot - [ttf2eot](https://github.com/fontello/ttf2eot).
     */
    formatOptions?: { [format in T]?: unknown };
    /**
     * An array of globs, of the SVG files to add into the webfont
     * @default ['*.svg']
     */
    files?: string[];
    /**
     * Font file types to generate. Possible values: `svg`, `ttf`, `woff`, `woff2`, `eot`.
     * @default ['eot', 'woff', 'woff2', 'ttf', 'svg']
     */
    types?: T | T[];
    /** Specific codepoints for certain icons. Icons without codepoints will have codepoints incremented from startCodepoint skipping duplicates. */
    codepoints?: { [key: string]: number };
    /** The outputted font height (defaults to the height of the highest input icon). */
    fontHeight?: number;
    /**
     * CSS class prefix for each of the generated icons.
     * @default 'icon-'
     */
    classPrefix?: string;
    /**
     * CSS base selector to which the font will be applied.
     * @default '.icon'
     */
    baseSelector?: string;
}

function parseIconTypesOption<T extends GeneratedFontTypes = GeneratedFontTypes>({ types }: IconPluginOptions<T>): T[] {
    if (Array.isArray(types)) {
        return types;
    }
    if (types) {
        return [types];
    }
    return ['eot', 'woff', 'woff2', 'ttf', 'svg'] as T[];
}

export function parseFiles({ files, context }: IconPluginOptions) {
    files ||= ['*.svg'];
    return files.flatMap(fileGlob => sync(fileGlob, { cwd: context })).map(file => `${context}/${file}`);
}

export function parseOptions<T extends GeneratedFontTypes = GeneratedFontTypes>(options: IconPluginOptions<T>): WebfontsGeneratorOptions<T> {
    const formats = parseIconTypesOption<T>(options);
    const files = parseFiles(options);
    options.dest ||= resolve(options.context, '..', 'artifacts');
    options.fontName ||= 'iconfont';
    return {
        files,
        types: formats,
        order: formats,
        fontName: options.fontName,
        fontHeight: options.fontHeight || 1000, // Fixes conversion issues with small svgs,
        codepoints: options.codepoints || {},
        templateOptions: {
            baseSelector: options.baseSelector || '.icon',
            classPrefix: options.classPrefix ?? 'icon-',
        },
        html: Boolean(options.html || options.htmlDest),
        css: Boolean(options.css || options.cssDest),
        ligature: options.ligature ?? true,
        writeFiles: options.writeFiles ?? false,
        formatOptions: options.formatOptions || {},
        dest: options.dest.endsWith('/') ? options.dest : `${options.dest}/`,
        ...(options.cssDest && { cssDest: resolve(options.dest, options.fontName.toLowerCase() + '.css') }),
        ...(options.cssTemplate && { cssTemplate: resolve(options.dest, options.cssTemplate) }),
        ...(options.cssFontsUrl && { cssFontsUrl: resolve(options.dest, options.cssFontsUrl) }),
        ...(options.htmlTemplate && { htmlTemplate: resolve(options.dest, options.htmlTemplate) }),
        ...(options.htmlDest && { htmlDest: resolve(options.dest, options.htmlDest) }),
        ...(typeof options.fixedWidth !== 'undefined' && { fixedWidth: options.fixedWidth }),
        ...(typeof options.centerHorizontally !== 'undefined' && { centerHorizontally: options.centerHorizontally }),
        ...(typeof options.normalize !== 'undefined' && { normalize: options.normalize }),
        ...(typeof options.round !== 'undefined' && { round: options.round }),
        ...(typeof options.descent !== 'undefined' && { descent: options.descent }),
    };
}
