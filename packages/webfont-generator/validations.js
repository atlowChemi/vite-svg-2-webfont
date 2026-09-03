function validateOptions(options) {
    if (!options.dest) throw new Error('"options.dest" is empty.');
    if (options.cssTemplate === '') throw new Error('"options.cssTemplate" must not be empty.');
    if (options.htmlTemplate === '') throw new Error('"options.htmlTemplate" must not be empty.');
    const types = options.types ?? ['eot', 'woff', 'woff2'];
    const invalidOrder = options.order?.find(type => !types.includes(type));
    if (invalidOrder) throw new Error(`Invalid font type order: '${invalidOrder}' is not present in 'types'.`);

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

export { validateOptions };
