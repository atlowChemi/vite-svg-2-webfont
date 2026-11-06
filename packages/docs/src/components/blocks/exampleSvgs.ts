const sources = import.meta.glob<string>(['../../fonts/examples/icons/*.svg', '../../fonts/examples/*/icons/**/*.svg'], { query: '?raw', import: 'default', eager: true });

/** One input SVG of an example, keyed by its path inside `icons/`. */
export interface ExampleIcon {
    example: string;
    file: string;
    svg: string;
}

/** Key for the shared `examples/icons/` folder, used by every example without its own `icons/`. */
const SHARED = '';

export const sourceMap = Object.groupBy(
    Object.entries(sources)
        .map(([path, svg]) => {
            const example = path.match(/^\.\.\/\.\.\/fonts\/examples\/([^/]+)\/icons\//)?.[1] ?? SHARED;
            const folder = example === SHARED ? '../../fonts/examples/icons/' : `../../fonts/examples/${example}/icons/`;
            return { example, file: path.slice(folder.length), svg } satisfies ExampleIcon;
        })
        .toSorted((a, b) => a.file.localeCompare(b.file)),
    ({ example }) => example,
);

/** An example's input SVGs: its own `icons/` folder when it has one, otherwise the shared folder. */
export const iconsFor = (example: string): ExampleIcon[] => sourceMap[example] ?? sourceMap[SHARED]!;
