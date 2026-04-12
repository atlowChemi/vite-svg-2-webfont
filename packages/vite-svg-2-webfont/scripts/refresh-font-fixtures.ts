import { extname, resolve } from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { build, normalizePath } from 'vite';

type ViteBuildResult = Awaited<ReturnType<typeof build>>;
type RolldownOutput = Extract<ViteBuildResult, { output: unknown }>;
type OutputAsset = Extract<RolldownOutput['output'][1], { type: 'asset' }>;

const rootDir = process.cwd();
const fixturesRoot = resolve(rootDir, 'src/fixtures');
const fontsDir = resolve(fixturesRoot, 'fonts');
const configFile = normalizePath(resolve(fixturesRoot, 'vite.no-inline.config.ts'));
const expectedTypes = ['svg', 'eot', 'woff', 'woff2', 'ttf'];

function getBuildOutput(result: ViteBuildResult) {
    if (Array.isArray(result)) {
        return result[0]?.output;
    }
    return 'output' in result ? result.output : undefined;
}

function getAssetSource(asset: OutputAsset) {
    if (typeof asset.source === 'string') {
        return asset.source;
    }
    return Buffer.from(asset.source);
}

const buildResult = await build({
    configFile,
    logLevel: 'silent',
    root: fixturesRoot,
});

const output = getBuildOutput(buildResult);
if (!output) {
    throw new Error('Unexpected Vite build result while refreshing font fixtures.');
}

await mkdir(fontsDir, { recursive: true });

for (const type of expectedTypes) {
    const asset = output.find((chunk): chunk is OutputAsset => chunk.type === 'asset' && chunk.fileName.startsWith('assets/iconfont-') && extname(chunk.fileName) === `.${type}`);

    if (!asset) {
        throw new Error(`Expected emitted ${type} asset while refreshing fixtures.`);
    }

    const destination = resolve(fontsDir, `iconfont.${type}`);
    // oxlint-disable-next-line no-await-in-loop -- this is a dev script, readability is more important than performance
    await writeFile(destination, getAssetSource(asset));
}

console.log('Refreshed font fixtures in src/fixtures/fonts');
