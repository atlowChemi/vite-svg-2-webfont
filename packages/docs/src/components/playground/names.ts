/** Characters the engine accepts in glyph names: Unicode letters and numbers, hyphens, and underscores. */
const VALID_NAME = /^[\p{L}\p{N}_-]+$/u;
const INVALID_RUN = /[^\p{L}\p{N}_-]+/gu;

export function isValidName(name: string): boolean {
    return VALID_NAME.test(name);
}

/** Derive an icon name from a file name, replacing characters the engine rejects with hyphens. */
export function iconName(fileName: string): string {
    const name = fileName
        .replace(/\.svg$/i, '')
        .replaceAll(INVALID_RUN, '-')
        .replaceAll(/^-+|-+$/g, '');
    return name || 'icon';
}

/** `name`, or `name-2`, `name-3`, … when it is already taken. */
export function uniqueName(name: string, taken: ReadonlySet<string>): string {
    if (!taken.has(name)) return name;
    let suffix = 2;
    while (taken.has(`${name}-${suffix}`)) suffix++;
    return `${name}-${suffix}`;
}
