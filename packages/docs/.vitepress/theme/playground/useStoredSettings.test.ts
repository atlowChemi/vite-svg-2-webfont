import { describe, expect, it } from 'vite-plus/test';
import { defaultSettings } from './options';
import { useStoredSettings } from './useStoredSettings';

describe('useStoredSettings', () => {
    it('never changes the shared defaults when settings change in place', () => {
        const { settings, reset } = useStoredSettings();
        settings.value.types.push('ttf');
        reset();
        settings.value.colorNames.push('add');
        settings.value.order.reverse();
        expect(defaultSettings.types).toEqual(['woff2', 'woff']);
        expect(defaultSettings.colorNames).toEqual([]);
        expect(defaultSettings.order).toEqual(['eot', 'woff2', 'woff', 'ttf', 'svg']);
    });
});
