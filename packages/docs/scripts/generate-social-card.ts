import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
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

// Render with bundled Open Sans only: system fonts differ between machines, and CI's fallback is wide enough to push
// the title past the card's edge.
const openSansDir = dirname(createRequire(import.meta.url).resolve('@expo-google-fonts/open-sans/package.json'));
const font = {
    fontFiles: ['500Medium/OpenSans_500Medium', '600SemiBold/OpenSans_600SemiBold', '700Bold/OpenSans_700Bold', '800ExtraBold/OpenSans_800ExtraBold'].map(file =>
        resolve(openSansDir, `${file}.ttf`),
    ),
    loadSystemFonts: false,
    defaultFontFamily: 'Open Sans',
};

await Promise.all(
    cards.map(async ({ fileName, width }) => {
        const svg = await readFile(resolve(rootDir, `${fileName}.svg`));
        const png = new Resvg(svg, { font, fitTo: { mode: 'width', value: width } }).render().asPng();
        const outputPath = resolve(rootDir, `${fileName}.png`);
        await writeFile(outputPath, png);
        console.log(`Generated ${outputPath}`);
    }),
);
