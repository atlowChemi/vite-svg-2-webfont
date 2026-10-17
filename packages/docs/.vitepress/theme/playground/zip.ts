import type { CollectedOutputs } from '@atlowchemi/webfont-browser-generator';
import { zipSync } from 'fflate';

/** File name for an output kind. The generated CSS and HTML refer to fonts by these names. */
export function outputFileName(fontName: string, kind: string): string {
    return `${fontName}.${kind}`;
}

/** Already-compressed formats are stored as-is; everything else is deflated. */
const STORED = new Set(['woff', 'woff2']);

/** Every output in one ZIP, named so the CSS and HTML work once extracted. */
export function zipOutputs(fontName: string, outputs: CollectedOutputs): Uint8Array {
    return zipSync(Object.fromEntries(Object.entries(outputs).map(([kind, bytes]) => [outputFileName(fontName, kind), [bytes, { level: STORED.has(kind) ? 0 : 6 }]])));
}
