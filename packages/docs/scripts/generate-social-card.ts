import { resolve } from 'node:path';
import { readFile, writeFile } from 'node:fs/promises';
import { Resvg } from '@resvg/resvg-js';

/**
 * Social images rendered from the optimized SVGs in `public/`: the docs site's Open Graph card, and the repository's
 * GitHub social preview (uploaded by hand in the repository settings; GitHub recommends 1280×640).
 */
const cards = [
    { fileName: 'social-card', width: 1200 },
    { fileName: 'github-social-preview', width: 1280 },
];
const rootDir = resolve(import.meta.dirname, '..', 'public');

// The SVGs outline their text (see `generate-brand.ts`), so no fonts are needed; skipping system fonts keeps renders
// identical across machines.
const font = { loadSystemFonts: false };

await Promise.all(
    cards.map(async ({ fileName, width }) => {
        const svg = await readFile(resolve(rootDir, `${fileName}.svg`));
        const png = new Resvg(svg, { font, fitTo: { mode: 'width', value: width } }).render().asPng();
        const outputPath = resolve(rootDir, `${fileName}.png`);
        await writeFile(outputPath, png);
        console.log(`Generated ${outputPath}`);
    }),
);
