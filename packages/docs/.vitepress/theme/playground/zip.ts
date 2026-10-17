import type { CollectedOutputs } from '@atlowchemi/webfont-browser-generator';
import { type AsyncZippable, zip } from 'fflate';

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
