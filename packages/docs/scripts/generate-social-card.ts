import { resolve } from 'node:path';
import { readFile, writeFile } from 'node:fs/promises';
import { Resvg } from '@resvg/resvg-js';

const fileName = 'social-card';
const rootDir = resolve(import.meta.dirname, '..', 'public');
const inputPath = resolve(rootDir, `${fileName}.svg`);
const outputPath = resolve(rootDir, `${fileName}.png`);

const svg = await readFile(inputPath);
const resvg = new Resvg(svg, {
    fitTo: {
        mode: 'width',
        value: 1200,
    },
});
const png = resvg.render().asPng();

await writeFile(outputPath, png);

console.log(`Generated ${outputPath}`);
