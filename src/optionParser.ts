import glob from 'glob';
import { resolve } from 'path';
import { hasFileExtension } from './utils';
import { InvalidWriteFilesTypeError, NoIconsAvailableError } from './errors';
import type { WebfontsGeneratorOptions, GeneratedFontTypes } from '@vusion/webfonts-generator';

const { sync } = glob;

const FILE_TYPE_OPTIONS = ['html', 'css', 'fonts'] as const;
type FileType = (typeof FILE_TYPE_OPTIONS)[number];

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
     * - Relative to the {@link dest} property, unless set to an absolute path.
     * - Postfixed with {@link fontName} unless set to a file name with a file extension.
     * @default path.join(options.dest, options.fontName + '.css')
     */
    cssDest?: string;
    /**
     * Path of custom CSS template. Generator uses handlebars templates.
     * Template receives options from {@link WebfontsGeneratorOptions.templateOptions `templateOptions`} along with the following options:
     * - {@link fontName} `string`
     * - src `string` – Value of the `src` property for `@font-face`.
     * - {@link codepoints} `object` – Codepoints of icons in hex format.
     *
     * Paths of default templates are stored in the `templates` object.
     * - `templates.css` – Default CSS template path. It generates classes with names based on values from {@link WebfontsGeneratorOptions.templateOptions `templateOptions`}.
     * - `templates.scss` – Default SCSS template path. It generates mixin `webfont-icon` to add icon styles. It is safe to use multiple generated files with mixins together.
     */
    cssTemplate?: string;
    /**
     * Fonts path used in CSS file.
     * @default options.cssDest
     */
    cssFontsUrl?: string;
    /**
     * Path for generated HTML file.
     * - Relative to the {@link dest} property, unless set to an absolute path.
     * - Postfixed with {@link fontName} unless set to a file name with a file extension.
     * @default path.join(options.dest, options.fontName + '.html')
     */
    htmlDest?: string;
    /**
     * HTML template path. Generator uses handlebars templates.
     *
     * Template receives options from {@link WebfontsGeneratorOptions.templateOptions `templateOptions`} along with the following options:
     * - {@link fontName} `string`
     * - styles `string` – Styles generated with default CSS template. ({@link cssFontsPath `cssFontsPath`} is changed to relative path from {@link htmlDest `htmlDest`} to {@link dest `dest`})
     * - names `string[]` – Names of icons.
     */
    htmlTemplate?: string;
    /**
     * Sets the type of file to be saved to system during development.
     *
     * `true` will generate all, and `false` will generate no files.
     * @default false
     */
    generateFiles?: boolean | FileType | FileType[];
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

export function parseIconTypesOption<T extends GeneratedFontTypes = GeneratedFontTypes>({ types }: Pick<IconPluginOptions<T>, 'types'>): T[] {
    if (Array.isArray(types)) {
        return types;
    }
    if (types) {
        return [types];
    }
    return ['eot', 'woff', 'woff2', 'ttf', 'svg'] as T[];
}

export function parseFiles({ files, context }: Pick<IconPluginOptions, 'files' | 'context'>) {
    files ||= ['*.svg'];
    const resolvedFiles = files.flatMap(fileGlob => sync(fileGlob, { cwd: context })).map(file => `${context}/${file}`);
    if (!resolvedFiles.length) {
        throw new NoIconsAvailableError('The specified file globs did not resolve any files in the context.');
    }
    return resolvedFiles;
}

export function resolveFileDest(globalDest: string, fileDest: string | undefined, fontName: string, extension: 'css' | 'html') {
    if (!fileDest) {
        return resolve(globalDest, `${fontName.toLowerCase()}.${extension}`);
    }
    if (hasFileExtension(fileDest)) {
        return resolve(globalDest, fileDest);
    }
    return resolve(globalDest, fileDest, `${fontName.toLowerCase()}.${extension}`);
}

export function buildFileTypeList({ generateFiles }: Pick<IconPluginOptions, 'generateFiles'>): readonly FileType[] {
    if (!generateFiles || typeof generateFiles === 'boolean') {
        return generateFiles ? FILE_TYPE_OPTIONS : [];
    }
    if (!Array.isArray(generateFiles)) {
        generateFiles = [generateFiles];
    }
    const invalidTypes = generateFiles.filter(type => !FILE_TYPE_OPTIONS.includes(type));
    if (invalidTypes.length) {
        throw new InvalidWriteFilesTypeError(invalidTypes);
    }
    return generateFiles;
}

export function parseGenerateFilesOption(options: Pick<IconPluginOptions, 'generateFiles'>) {
    const fileTypes = new Set(buildFileTypeList(options));
    return {
        fonts: fileTypes.has('fonts'),
        html: fileTypes.has('html'),
        css: fileTypes.has('css'),
    };
}

export function parseOptions<T extends GeneratedFontTypes = GeneratedFontTypes>(options: IconPluginOptions<T>) {
    const formats = parseIconTypesOption<T>(options);
    const files = parseFiles(options);
    const generateFilesOptions = parseGenerateFilesOption(options);
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
        html: generateFilesOptions.html,
        css: generateFilesOptions.css,
        ligature: options.ligature ?? true,
        formatOptions: options.formatOptions || {},
        dest: options.dest.endsWith('/') ? options.dest : `${options.dest}/`,
        writeFiles: generateFilesOptions.fonts,
        cssDest: resolveFileDest(options.dest, options.cssDest, options.fontName, 'css'),
        htmlDest: resolveFileDest(options.dest, options.htmlDest, options.fontName, 'html'),
        ...(options.cssTemplate && { cssTemplate: resolve(options.dest, options.cssTemplate) }),
        ...(options.cssFontsUrl && { cssFontsUrl: resolve(options.dest, options.cssFontsUrl) }),
        ...(options.htmlTemplate && { htmlTemplate: resolve(options.dest, options.htmlTemplate) }),
        ...(typeof options.fixedWidth !== 'undefined' && { fixedWidth: options.fixedWidth }),
        ...(typeof options.centerHorizontally !== 'undefined' && { centerHorizontally: options.centerHorizontally }),
        ...(typeof options.normalize !== 'undefined' && { normalize: options.normalize }),
        ...(typeof options.round !== 'undefined' && { round: options.round }),
        ...(typeof options.descent !== 'undefined' && { descent: options.descent }),
    } satisfies WebfontsGeneratorOptions<T>;
}
