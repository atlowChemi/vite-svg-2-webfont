import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { expect, it } from 'vitest';

// A CSS module's import is typed as any string key, so a misspelled class would render as `undefined` and silently lose
// its styles. Every `styles.name` the code reads must be a class its module defines.

const src = import.meta.dirname;
const files = (dir: string): string[] => readdirSync(dir, { withFileTypes: true }).flatMap(entry => (entry.isDirectory() ? files(join(dir, entry.name)) : [join(dir, entry.name)]));

/** The classes a module defines: its selectors' class names, outside `:global(…)`, comments and strings. */
function classesOf(css: string): Set<string> {
    const local = css
        .replaceAll(/\/\*[\s\S]*?\*\//g, '')
        .replaceAll(/(["'])(?:\\.|(?!\1).)*\1/g, '')
        .replaceAll(/:global\([^)]*\)/g, '');
    return new Set([...local.matchAll(/\.(-?[_a-zA-Z][\w-]*)/g)].map(m => m[1]!));
}

const uses = files(src)
    .filter(file => /\.tsx?$/.test(file) && !file.endsWith('.test.ts'))
    .flatMap(file => {
        const code = readFileSync(file, 'utf8');
        return code
            .matchAll(/^import (\w+) from '(\.[^']+\.module\.css)';$/gm)
            .flatMap(([, name, path]) => {
                const module = resolve(dirname(file), path!);
                const body = code.replaceAll(/^import .*$/gm, '');
                const used = new Set(body.matchAll(new RegExp(`\\b${name}\\.(\\w+)`, 'g')).map(m => m[1]!));
                return [{ file: relative(src, file), module: relative(src, module), classes: classesOf(readFileSync(module, 'utf8')), used }];
            })
            .toArray();
    });

it('finds the modules the code imports', () => {
    expect(uses.length).toBeGreaterThan(20);
});

it.each(uses.map(use => [`${use.file} → ${use.module}`, use] as const))('%s reads only classes the module defines', (_, { classes, used }) => {
    expect(
        used
            .values()
            .filter(name => !classes.has(name))
            .toArray(),
    ).toEqual([]);
});
