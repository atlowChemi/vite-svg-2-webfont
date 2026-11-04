import { describe, expect, it } from 'vite-plus/test';
import { defaultSettings } from './options.ts';
import { movedFormat, parseOptionalNumber, toggled } from './settingsEdits.ts';

describe('settings edits', () => {
    it.each([
        ['', undefined],
        ['  ', undefined],
        ['12', 12],
        ['-3.5', -3.5],
        ['abc', undefined],
    ])('reads the number field %j as %j', (text, value) => {
        expect(parseOptionalNumber(text)).toBe(value);
    });

    it('adds and removes list items without changing the given list', () => {
        const list = ['woff2', 'woff'];
        expect(toggled(list, 'ttf', true)).toEqual(['woff2', 'woff', 'ttf']);
        expect(toggled(list, 'woff2', false)).toEqual(['woff']);
        expect(toggled(list, 'woff2', true)).toBe(list);
        expect(toggled(list, 'ttf', false)).toBe(list);
        expect(list).toEqual(['woff2', 'woff']);
    });

    it('moves a selected format past unselected ones', () => {
        const settings = { ...defaultSettings, types: ['woff2', 'svg'] as const };
        // eot, woff2, woff, ttf, svg: moving svg up swaps it with woff2, the selected format before it.
        expect(movedFormat({ ...settings, types: [...settings.types] }, 'svg', -1)).toEqual(['eot', 'svg', 'woff', 'ttf', 'woff2']);
        expect(movedFormat({ ...settings, types: [...settings.types] }, 'woff2', -1)).toBeUndefined();
        expect(movedFormat({ ...settings, types: [...settings.types] }, 'svg', 1)).toBeUndefined();
        expect(defaultSettings.order).toEqual(['eot', 'woff2', 'woff', 'ttf', 'svg']);
    });
});
