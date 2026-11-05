import { createHash } from 'node:crypto';
import { basename, join, relative } from 'node:path';
import type { Plugin } from 'vite-plus';

const docsRoot = join(import.meta.dirname, '..');

/**
 * A CSS module class's name: its file, its own name, and a hash of the file's path. The renderer (built for Node) and the
 * browser entries import the same modules in separate builds, so names depend on nothing but the path and the class.
 */
export const scopedName = (name: string, file: string) => {
    const path = relative(docsRoot, file.split('?', 1)[0]!);
    const stem = basename(path).replace(/\.module\.css$/, '');
    return `${stem}_${name}_${createHash('sha256').update(path).digest('base64url').slice(0, 5)}`;
};

/**
 * Vite drops a CSS module that is imported only for its styles, as if it were unused JS. Each entry imports the modules
 * its page's components render with, so their styles must stay.
 */
export const keepCssModules = (): Plugin => ({
    name: 'keep-css-modules',
    enforce: 'post',
    transform(code, id) {
        if (/\.module\.css(?:\?|$)/.test(id)) return { code, map: null, moduleSideEffects: 'no-treeshake' };
        return undefined;
    },
});
