import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vite-plus/test';
import { checkTabCount, maxTabs } from './codeTabs.ts';

const css = readFileSync(join(import.meta.dirname, 'codeTabs.module.css'), 'utf8');

describe('code tabs', () => {
    it('hide the panel at every position up to the limit, and no further', () => {
        const positions = [...css.matchAll(/input:nth-of-type\((\d+)\):checked\)\) > \.panels > :nth-child\((\d+)\)/g)].map(([, tab, panel]) => {
            expect(panel).toBe(tab);
            return Number(tab);
        });
        expect(positions).toEqual(Array.from({ length: maxTabs }, (_, i) => i + 1));
    });

    it('fail the build on a group with more tabs than that', () => {
        expect(() => checkTabCount(maxTabs, 'A code group')).not.toThrow();
        expect(() => checkTabCount(maxTabs + 1, 'A code group')).toThrow(`A code group has ${maxTabs + 1} tabs`);
    });
});
