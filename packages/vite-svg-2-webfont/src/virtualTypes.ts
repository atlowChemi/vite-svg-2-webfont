import { hash, randomUUID } from 'node:crypto';
import { mkdir, readFile, readdir, rename, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createLock } from './utils';

export const declarationDirectory: string = fileURLToPath(new URL('./generated/', import.meta.url));

/** Strip only the two type constructs in the engine's fixed output; string contents stay escaped. */
export function moduleSource(source: string): string {
    return source.replaceAll(' as const;\n', ';\n').replace('export type IconName = (typeof icons)[number];\n', '');
}

/** The engine's fixed TS contract, with initializers converted to literal declarations. */
export function moduleDeclaration(id: string, source: string): string {
    const declarations = source.replace(
        /^export const (\w+) = (.*?) as const;\n/gms,
        (_, name: string, value: string) => `export const ${name}: ${name === 'icons' ? 'readonly ' : ''}${value};\n`,
    );
    return `declare module ${JSON.stringify(id)} {\n${declarations}}\n`;
}

async function writeChanged(path: string, content: string) {
    if ((await readFile(path, 'utf8').catch(() => undefined)) === content) return;
    const temporary = `${path}.${randomUUID()}.tmp`;
    try {
        await writeFile(temporary, content);
        await rename(temporary, path);
    } finally {
        await rm(temporary, { force: true });
    }
}

/** Serialize publication across processes; never publish an index pointing at missing files. */
export async function publishDeclaration(id: string, source: string, directory: string = declarationDirectory): Promise<void> {
    await mkdir(directory, { recursive: true });
    await using _lock = await createLock(directory);
    const name = `${hash('sha256', id)}.d.ts`;
    const [listing, writing] = await Promise.allSettled([readdir(directory), writeChanged(join(directory, name), moduleDeclaration(id, source))]);
    if (listing.status === 'rejected') throw listing.reason;
    if (writing.status === 'rejected') throw writing.reason;
    const references = new Set(listing.value)
        .add(name)
        .values()
        .filter(file => /^[a-f0-9]{64}\.d\.ts$/.test(file))
        .map(file => `/// <reference path="./${file}" />\n`)
        .toArray()
        // oxlint-disable-next-line unicorn/no-array-sort -- This array was just allocated by toArray().
        .sort()
        .join('');
    await writeChanged(join(directory, 'index.d.ts'), references);
}
