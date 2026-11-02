import { readFileSync } from 'node:fs';
import { basename, dirname } from 'node:path';
import { optionKey } from './optionKey';

// The same option under each tool's name (cssDest, css_dest), for the specimen cards' "Also in" links.
export interface OptionEntry {
    tool: string;
    name: string;
    id: string;
}
export interface OptionTools {
    [key: string]: OptionEntry[];
}
declare const data: OptionTools;
export { data };

// VitePress's heading slug, so links land on the heading's id.
const slug = (heading: string) =>
    heading
        .replace(/[\s~`!@#$%^&*()\-_+=[\]{}|\\;:"'“”‘’<>,.?/]+/g, '-')
        .replace(/^-+|-+$/g, '')
        .replace(/^(\d)/, '_$1')
        .toLowerCase();

export default {
    watch: ['../../../*/configuration.md'],
    load(files: string[]): OptionTools {
        const options: OptionTools = {};
        for (const file of files) {
            const tool = basename(dirname(file));
            for (const [, heading] of readFileSync(file, 'utf8').matchAll(/^### `([^`]+)`\s*$/gm)) {
                (options[optionKey(heading!)] ??= []).push({ tool, name: heading!, id: slug(heading!) });
            }
        }
        return options;
    },
};
