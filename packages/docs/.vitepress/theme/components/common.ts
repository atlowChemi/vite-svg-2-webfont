const sources = import.meta.glob<string>(['../../../examples/icons/*.svg', '../../../examples/*/icons/**/*.svg'], { query: '?raw', import: 'default', eager: true });

/** Key for the shared `examples/icons/` folder, used by every example without its own `icons/`. */
const SHARED = '';

export const sourceMap = Object.groupBy(
    Object.entries(sources)
        .map(([path, svg]) => {
            const example = path.match(/\.{2}\/\.{2}\/\.{2}\/examples\/([^/]+)\/icons\//)?.[1] ?? SHARED;
            const folder = example === SHARED ? '../../../examples/icons/' : `../../../examples/${example}/icons/`;
            return { example, file: path.slice(folder.length), svg };
        })
        .toSorted((a, b) => a.file.localeCompare(b.file)),
    ({ example }) => example,
);

/** An example's input SVGs: its own `icons/` folder when it has one, otherwise the shared folder. */
export const iconsFor = (example: string) => sourceMap[example] ?? sourceMap[SHARED]!;
