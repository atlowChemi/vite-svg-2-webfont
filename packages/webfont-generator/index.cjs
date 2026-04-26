const templates = require('./templates.cjs');

const UPSTREAM_TTF_COMPAT_TS = 1_484_141_760_000;

function coerceCodepoints(codepoints) {
    if (!codepoints) return undefined;
    return Object.fromEntries(Object.entries(codepoints).map(([name, value]) => [name, String.fromCharCode(value).codePointAt(0) ?? 0]));
}

async function generateWebfonts(options) {
    if (!options.dest) throw new Error('"options.dest" is empty.');
    if (!options.files?.length) throw new Error('"options.files" is empty.');

    const { rename, cssContext, htmlContext, ...nativeFields } = options;

    const nativeOptions = {
        ...nativeFields,
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

    const { generateWebfonts: generateNativeBinding } = await import('./binding.js');

    return generateNativeBinding(
        nativeOptions,
        rename,
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

module.exports = { generateWebfonts, templates };
