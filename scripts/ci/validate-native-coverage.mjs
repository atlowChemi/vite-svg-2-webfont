import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const file = process.argv[2];
const mode = process.argv[3] ?? 'native';
const root = `${resolve(import.meta.dirname, '../..').replaceAll('\\', '/')}/`;
const text = readFileSync(file, 'utf8').replace(/^SF:(.*)$/gm, (_, source) => {
    const path = source.replaceAll('\\', '/');
    const relative = path.startsWith(root) ? path.slice(root.length) : path;
    return `SF:${mode === 'js' && !relative.startsWith('packages/') ? `packages/webfont-generator/${relative}` : relative}`;
});
const prefixes =
    mode === 'native'
        ? ['crates/webfont-generator/src/', 'packages/webfont-generator/native/']
        : mode === 'adapter'
          ? ['packages/webfont-generator/native/']
          : mode === 'js'
            ? ['packages/webfont-generator/']
            : ['crates/webfont-generator/src/'];
for (const prefix of prefixes) {
    const records = text.split('end_of_record').filter(record => record.includes(`SF:${prefix}`));
    const hits = records.reduce((sum, record) => sum + Number(record.match(/^LH:(\d+)$/m)?.[1] ?? 0), 0);
    if (!records.length || !hits) throw new Error(`Native coverage is missing executed lines for ${prefix}`);
    console.log(`${prefix}: ${records.length} files, ${hits} covered lines`);
}
writeFileSync(file, text);
