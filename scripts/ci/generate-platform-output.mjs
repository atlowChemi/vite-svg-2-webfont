/* eslint-disable no-await-in-loop -- Run corpus cases sequentially to bound native build memory across CI runners. */
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { generateWebfonts } from '../../packages/webfont-generator/index.js';

// Run from the repo root. Forward-slash relative paths are deliberately identical on every OS.
const root = 'artifacts/platform-output';
const output = `${root}/${process.platform}-${process.arch}`;
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
const fixtures = 'crates/webfont-generator/src/svg/fixtures';
const files = [
    'icons/cleanicons/search.svg',
    'icons/pathfillnone/account.svg',
    'icons/transformedicons/arrow-left.svg',
    'icons/rotatedrectangle/rotatedrectangle.svg',
    'icons/roundedcorners/roundedrect.svg',
    'icons/translatex/translatex.svg',
    'winding/nested-circles.svg',
    'winding/evenodd-hole.svg',
].map(file => `${fixtures}/${file}`);

// Small layered paint fixtures; geometry coverage comes from the existing engine corpus above.
for (const [name, color, inset] of [
    ['light', '#e23', 20],
    ['bold', '#36c', 10],
]) {
    await mkdir(`${root}/inputs/${name}`, { recursive: true });
    await writeFile(
        `${root}/inputs/${name}/paint.svg`,
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path fill="${color}" d="M0 0H100V100H0Z"/><path fill="currentColor" opacity="0.5" d="M${inset} 20H80V80H${inset}Z"/></svg>`,
    );
}
const modern = ['ttf', 'woff', 'woff2'];
const variants = ['light', 'bold'].map((name, index) => ({
    name,
    files: [`${root}/inputs/${name}/paint.svg`, ...files],
    weight: index ? 700 : 300,
    default: index === 0,
}));
const cases = [
    { name: 'ordinary', files, types: ['svg', 'ttf', 'eot', 'woff', 'woff2'] },
    { name: 'optimized', files, types: ['svg', ...modern], optimizeOutput: true },
    { name: 'woff2-q9', files, types: modern, formatOptions: { woff2: { compressionQuality: 9 } } },
    { name: 'color', files: variants[0].files, types: modern, colorGlyphs: ['paint'] },
    { name: 'variants', variants, types: modern },
    { name: 'color-variants', variants, types: modern, colorGlyphs: ['paint'] },
];

async function save(result, name, mode, types) {
    const directory = `${output}/${mode}/${name}`;
    await mkdir(directory, { recursive: true });
    for (const type of types) {
        assert.ok(result[type] !== null, `${name}: missing ${type}`);
        await writeFile(`${directory}/font.${type}`, result[type]);
    }
    await writeFile(`${directory}/font.css`, result.generateCss());
    await writeFile(`${directory}/font.html`, result.generateHtml());
}

for (const { name, ...input } of cases) {
    // The public wrapper supplies its compatibility timestamp. Do not override it here.
    const options = { ...input, dest: `${root}/fonts`, fontName: 'parity', css: true, html: true, writeFiles: false, incremental: true };
    const fresh = await generateWebfonts(options);
    await save(fresh, name, 'fresh', input.types);
    const subset = input.variants ? { variants: input.variants.map(variant => ({ ...variant, files: variant.files.slice(0, -1) })) } : { files: input.files.slice(0, -1) };
    const initial = await generateWebfonts({ ...options, ...subset });
    const complete = input.variants ? { variants: input.variants.map(variant => ({ variant: variant.name, files: variant.files })) } : { files: input.files };
    const regenerated = await initial.regenerateAsync(complete);
    await save(regenerated, name, 'incremental', input.types);
}
await writeFile(
    `${output}/environment.json`,
    JSON.stringify(
        {
            platform: process.platform,
            arch: process.arch,
            node: process.version,
            rust: execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim(),
            revision: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
        },
        null,
        2,
    ),
);
console.log(`Generated platform comparison corpus in ${output}`);
