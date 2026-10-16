import { generateWebfonts as generateNativeBinding, MissingGlyphBehavior } from './binding.js';
import * as templates from './templates.js';
import { validateOptions } from './validations.js';

// @vusion/webfonts-generator always overwrites `formatOptions.ttf.ts` with this value (milliseconds passed as
// svg2ttf's seconds-based `ts`); the compatibility wrapper does the same so TTF output keeps matching upstream.
const UPSTREAM_TTF_COMPAT_TS = 1_484_141_760_000;

function coerceCodepoints(codepoints) {
    if (!codepoints) return undefined;
    return Object.fromEntries(Object.entries(codepoints).map(([name, value]) => [name, String.fromCharCode(value).codePointAt(0)]));
}

async function generateWebfonts(options) {
    validateOptions(options);

    const { rename, cssContext, htmlContext, ...nativeFields } = options;

    const nativeOptions = {
        ...nativeFields,
        files: options.files ?? [],
        codepoints: coerceCodepoints(options.codepoints),
        cssTemplate: options.cssTemplate,
        htmlTemplate: options.htmlTemplate,
        formatOptions: {
            ...options.formatOptions,
            ttf: {
                ...(typeof options.formatOptions?.ttf === 'object' && options.formatOptions.ttf),
                ts: UPSTREAM_TTF_COMPAT_TS,
            },
        },
    };

    return generateNativeBinding(
        nativeOptions,
        rename
            ? paths =>
                  paths.map(path => {
                      const name = rename(path);
                      if (typeof name !== 'string') throw new TypeError('rename callback must return a string');
                      return name;
                  })
            : undefined,
        cssContext
            ? context => {
                  cssContext(context);
                  return context;
              }
            : undefined,
        htmlContext
            ? context => {
                  htmlContext(context);
                  return context;
              }
            : undefined,
    );
}

generateWebfonts.templates = templates;

export { generateWebfonts, MissingGlyphBehavior, templates };
