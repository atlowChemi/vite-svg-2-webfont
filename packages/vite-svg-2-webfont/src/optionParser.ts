import { join, resolve, sep } from 'node:path';
import { globSync } from 'glob';
import { hasFileExtension } from './utils';
import { InvalidWriteFilesTypeError, NoIconsAvailableError } from './errors';
import type { CssContext, FontType, FormatOptions, GenerateWebfontsInputOptions } from '@atlowchemi/webfont-generator';
import type { IndexHtmlTransformContext } from 'vite';

const FILE_TYPE_OPTIONS = ['html', 'css', 'fonts'] as const;
type FileType = (typeof FILE_TYPE_OPTIONS)[number];

export interface IconPluginOptions<T extends FontType = FontType> {
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
    /** Calculate the bounds of a glyph and center it vertically. */
    centerVertically?: boolean;
    /**
     * Run an SVG path optimizer over each glyph before assembling the font.
     * Trades a small amount of build time for smaller output bytes.
     * @default false
     */
    optimizeOutput?: boolean;
    /**
     * Path for generated CSS file.
     * - Relative to the {@link dest} property, unless set to an absolute path.
     * - Postfixed with {@link fontName} unless set to a file name with a file extension.
     * @default path.join(options.dest, options.fontName + '.css')
     */
    cssDest?: string;
    /**
     * Path of a custom Handlebars CSS template. The rendering context provides the
     * fields documented on {@link CssContext} (`fontName`, `src`, `codepoints`),
     * and the plugin also forwards {@link baseSelector} and {@link classPrefix} into
     * it so default templates can read them.
     *
     * Paths of default templates are exposed via the `templates` export:
     * - `templates.css` – Default CSS template; generates classes prefixed with {@link classPrefix}.
     * - `templates.scss` – Default SCSS template; generates a `webfont-icon` mixin that's safe to combine with other generated mixin files.
     */
    cssTemplate?: string;
    /**
     * Hook for mutating the rendering context passed to the CSS template before the CSS file is generated.
     * The `context` object includes the named fields documented on {@link CssContext} (`fontName`, `src`, `codepoints`),
     * plus the {@link baseSelector} and {@link classPrefix} keys the plugin forwards to the underlying generator.
     */
    cssContext?: (context: CssContext) => void;
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
     * Path of a custom Handlebars HTML template. The rendering context provides
     * `fontName` `string`, `names` `string[]`, `codepoints` (as numbers), and
     * `styles` `string` — the pre-rendered CSS using the default CSS template,
     * with {@link cssFontsUrl} rewritten to a relative path from {@link htmlDest}
     * to {@link dest}. The plugin also forwards {@link baseSelector} and
     * {@link classPrefix} into the context.
     */
    htmlTemplate?: string;
    /**
     * Sets the type of files to be saved to system during development.
     *
     * `true` will generate all, and `false` will generate no files.
     * @default false
     */
    generateFiles?: boolean | FileType | FileType[];
    /**
     * Per-format options forwarded to the underlying webfont generator. See
     * {@link FormatOptions} — its `svg`, `ttf`, and `woff` fields each carry
     * their own typed and documented per-format options.
     */
    formatOptions?: FormatOptions;
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
    /**
     * Font file types to preload in build HTML output.
     *
     * Only generated formats can be preloaded, so values outside {@link types} are ignored.
     */
    preloadFormats?: NoInfer<T> | NoInfer<T>[];
    /**
     * Allows skipping preload tag injection for specific HTML entrypoints.
     */
    shouldProcessHtml?: (context: IndexHtmlTransformContext) => boolean;
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
    /**
     * Virtual module id which is used by Vite to import the plugin artifacts.
     * E.g. the default value is "vite-svg-2-webfont.css" so "virtual:vite-svg-2-webfont.css" should be imported.
     *
     * @default 'vite-svg-2-webfont.css'
     */
    moduleId?: string;
    /**
     * Inline font assets in CSS using base64 encoding.
     * @default false
     */
    inline?: boolean;
    /**
     * Allow outputting assets (HTML, CSS, and Fonts) during build.
     * @see {@link https://github.com/atlowChemi/vite-svg-2-webfont/issues/32#issuecomment-2203187501}
     * @default false
     */
    allowWriteFilesInBuild?: boolean;
}

function parseGeneratedFontTypeOption<T extends FontType = FontType>(types?: T | T[]): T[] {
    if (Array.isArray(types)) {
        return types;
    }
    if (types) {
        return [types];
    }
    return [];
}

export function parseIconTypesOption<T extends FontType = FontType>({ types }: Pick<IconPluginOptions<T>, 'types'>): T[] {
    const parsedTypes = parseGeneratedFontTypeOption(types);
    if (parsedTypes.length) {
        return parsedTypes;
    }
    return ['eot', 'woff', 'woff2', 'ttf', 'svg'] as T[];
}

export function parsePreloadFormatsOption<T extends FontType = FontType>({ preloadFormats }: Pick<IconPluginOptions<T>, 'preloadFormats'>): T[] {
    return parseGeneratedFontTypeOption(preloadFormats);
}

export function parseFiles({ files, context }: Pick<IconPluginOptions, 'files' | 'context'>): string[] {
    files ||= ['*.svg'];
    const resolvedFiles = globSync(files, { cwd: context })?.map(file => join(context, file)) || [];
    if (!resolvedFiles.length) {
        throw new NoIconsAvailableError('The specified file globs did not resolve any files in the context.');
    }
    return resolvedFiles;
}

export function resolveFileDest(globalDest: string, fileDest: string | undefined, fontName: string, extension: 'css' | 'html'): string {
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

export function parseGenerateFilesOption(options: Pick<IconPluginOptions, 'generateFiles'>): Record<'fonts' | 'html' | 'css', boolean> {
    const fileTypes = new Set(buildFileTypeList(options));
    return {
        fonts: fileTypes.has('fonts'),
        html: fileTypes.has('html'),
        css: fileTypes.has('css'),
    };
}

type RequiredKeys = 'fontHeight' | 'codepoints' | 'templateOptions' | 'html' | 'css' | 'ligature' | 'formatOptions' | 'writeFiles' | 'cssDest' | 'htmlDest' | 'types' | 'order';
interface ParsedOptions<T extends FontType = FontType> extends Omit<GenerateWebfontsInputOptions<T>, RequiredKeys>, Required<Pick<GenerateWebfontsInputOptions<T>, RequiredKeys>> {}

export function parseOptions<T extends FontType = FontType>(options: IconPluginOptions<T>): ParsedOptions<T> {
    const formats = parseIconTypesOption<T>(options);
    const files = parseFiles(options);
    const generateFilesOptions = parseGenerateFilesOption(options);
    const formatOptions = options.formatOptions;
    const svgFormatOptions = formatOptions?.svg;
    options.dest ||= resolve(options.context, '..', 'artifacts');
    options.fontName ||= 'iconfont';
    return {
        files,
        types: formats,
        order: formats,
        fontName: options.fontName,
        fontHeight: options.fontHeight || 1000, // Fixes conversion issues with small svgs,
        codepoints: options.codepoints || {},
        optimizeOutput: options.optimizeOutput ?? false,
        templateOptions: {
            baseSelector: options.baseSelector || '.icon',
            classPrefix: options.classPrefix ?? 'icon-',
        },
        html: generateFilesOptions.html,
        css: generateFilesOptions.css,
        ligature: options.ligature ?? true,
        formatOptions: {
            ...formatOptions,
            ...(typeof options.centerVertically !== 'undefined' && {
                svg: {
                    centerVertically: options.centerVertically,
                    ...(typeof svgFormatOptions === 'object' && svgFormatOptions),
                },
            }),
        },
        dest: `${options.dest.replace(/[/\\]$/, '')}${sep}`,
        writeFiles: generateFilesOptions.fonts,
        cssDest: resolveFileDest(options.dest, options.cssDest, options.fontName, 'css'),
        htmlDest: resolveFileDest(options.dest, options.htmlDest, options.fontName, 'html'),
        ...(options.cssTemplate && { cssTemplate: resolve(options.dest, options.cssTemplate) }),
        ...(options.cssContext && { cssContext: options.cssContext }),
        ...(options.cssFontsUrl && { cssFontsUrl: resolve(options.dest, options.cssFontsUrl) }),
        ...(options.htmlTemplate && { htmlTemplate: resolve(options.dest, options.htmlTemplate) }),
        ...(typeof options.fixedWidth !== 'undefined' && { fixedWidth: options.fixedWidth }),
        ...(typeof options.centerHorizontally !== 'undefined' && { centerHorizontally: options.centerHorizontally }),
        ...(typeof options.normalize !== 'undefined' && { normalize: options.normalize }),
        ...(typeof options.round !== 'undefined' && { round: options.round }),
        ...(typeof options.descent !== 'undefined' && { descent: options.descent }),
    } satisfies GenerateWebfontsInputOptions<T>;
}
