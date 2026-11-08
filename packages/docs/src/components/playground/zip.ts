import type { CollectedOutputs } from '@atlowchemi/webfont-browser-generator';
import { type AsyncZippable, zip } from 'fflate';
import { selectedTypes, type PlaygroundSettings } from './options.ts';

/** File name for an output kind. The generated CSS and HTML refer to fonts by these names. */
export function outputFileName(fontName: string, kind: string): string {
    return `${fontName}.${kind}`;
}

/** Already-compressed formats are stored as-is; everything else is deflated. */
const STORED = new Set(['woff', 'woff2']);

/** Every output in one ZIP, named so the CSS and HTML work once extracted. Compression runs in fflate's workers. */
export function zipOutputs(fontName: string, outputs: CollectedOutputs): Promise<Uint8Array<ArrayBuffer>> {
    const { promise, resolve, reject } = Promise.withResolvers<Uint8Array<ArrayBuffer>>();
    const files: AsyncZippable = Object.fromEntries(Object.entries(outputs).map(([kind, bytes]) => [outputFileName(fontName, kind), [bytes, { level: STORED.has(kind) ? 0 : 6 }]]));
    zip(files, (error, data) => (error ? reject(error) : resolve(data)));
    return promise;
}

/** The build's files in download order: fonts in their CSS order, then CSS, HTML and TypeScript. */
export function outputFiles(outputs: CollectedOutputs, settings: PlaygroundSettings) {
    const order: string[] = [...selectedTypes(settings), 'css', 'html', 'ts'];
    return Object.entries(outputs)
        .toSorted(([a], [b]) => order.indexOf(a) - order.indexOf(b))
        .map(([kind, bytes]) => ({
            kind,
            fileName: outputFileName(settings.fontName, kind),
            bytes,
            size: `${(bytes.byteLength / 1024).toFixed(1)} kB`,
        }));
}
