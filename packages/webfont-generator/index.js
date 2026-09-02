import { generateWebfonts as generateNativeBinding, MissingGlyphBehavior } from './binding.js';
import * as templates from './templates.js';

const UPSTREAM_TTF_COMPAT_TS = 1_484_141_760_000;

function coerceCodepoints(codepoints) {
    if (!codepoints) return undefined;
    return Object.fromEntries(Object.entries(codepoints).map(([name, value]) => [name, String.fromCharCode(value).codePointAt(0)]));
}

function validateOptions(options) {
    if (!options.dest) throw new Error('"options.dest" is empty.');

    const variants = options.variants;
    if (variants == null) {
        if (!options.files?.length) throw new Error('Either "options.files" or "options.variants" must be provided.');
        if (options.missingGlyphs != null) throw new Error('"options.missingGlyphs" requires "options.variants".');
        if (options.variantClassPrefix != null) throw new Error('"options.variantClassPrefix" requires "options.variants".');
        return;
    }

    if (options.files?.length) throw new Error('"options.files" must be empty when "options.variants" is provided.');
    if (variants.length < 2) throw new Error('"options.variants" must contain at least two variants.');
    if (options.types?.includes('svg')) throw new Error('"options.types" cannot include "svg" with "options.variants".');
    if (options.incremental === true) throw new Error('"options.incremental" cannot be true with "options.variants".');
    if (options.fontWeight != null) throw new Error('"options.fontWeight" cannot be used with "options.variants".');
    if (Object.hasOwn(options.templateOptions ?? {}, 'variantClassPrefix')) {
        throw new Error('"options.templateOptions.variantClassPrefix" cannot be used with "options.variants"; use "options.variantClassPrefix".');
    }

    const classPrefix = options.variantClassPrefix ?? 'icon--';
    if (typeof classPrefix !== 'string' || !classPrefix || /[\p{White_Space}\0]/u.test(classPrefix)) {
        throw new Error('"options.variantClassPrefix" must be non-empty and contain neither whitespace nor NUL.');
    }

    const names = new Set();
    let defaultCount = 0;
    for (const [index, variant] of variants.entries()) {
        const path = `options.variants[${index}]`;
        if (!variant.files?.length) throw new Error(`"${path}.files" is empty.`);
        if (!variant.name) throw new Error(`"${path}.name" is empty.`);
        if (/\p{White_Space}/u.test(variant.name)) throw new Error(`"${path}.name" contains whitespace.`);
        if (variant.name.includes('\0')) throw new Error(`"${path}.name" contains NUL.`);
        if (names.has(variant.name)) throw new Error(`"${path}.name" duplicates variant name "${variant.name}".`);
        names.add(variant.name);
        if (variant.weight != null && (!Number.isInteger(variant.weight) || variant.weight < 1 || variant.weight > 1000)) {
            throw new Error(`"${path}.weight" must be an integer between 1 and 1000, got ${variant.weight}.`);
        }
        defaultCount += Number(variant.default === true);
    }

    if (defaultCount !== 1) {
        throw new Error(`"options.variants" must contain exactly one default variant, found ${defaultCount}.`);
    }
    if (variants.every(variant => variant.weight != null)) {
        for (let index = 1; index < variants.length; index++) {
            if (variants[index - 1].weight >= variants[index].weight) {
                throw new Error(`"options.variants[${index}].weight" must be greater than the preceding explicit weight.`);
            }
        }
    }

    const missing = options.missingGlyphs;
    if (missing?.behavior === 'fallback') {
        if (missing.variant == null) {
            throw new Error('"options.missingGlyphs.variant" is required when behavior is "fallback".');
        }
        if (!names.has(missing.variant)) {
            throw new Error(`"options.missingGlyphs.variant" does not name a configured variant: "${missing.variant}".`);
        }
    } else if (missing?.variant != null) {
        throw new Error('"options.missingGlyphs.variant" is only valid when behavior is "fallback".');
    }
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
