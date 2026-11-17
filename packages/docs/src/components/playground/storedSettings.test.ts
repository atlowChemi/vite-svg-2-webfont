import { afterEach, describe, expect, it, vi } from 'vitest';
import { defaultSettings } from './options.ts';
import { freshSettings, loadSettings, saveSettings } from './useStoredSettings.ts';

afterEach(() => {
    vi.unstubAllGlobals();
});

/** A `localStorage` stand-in backed by a Map. */
function stubStorage() {
    const stored = new Map<string, string>();
    vi.stubGlobal('localStorage', { getItem: (key: string) => stored.get(key) ?? null, setItem: (key: string, value: string) => void stored.set(key, value) });
    return stored;
}

describe('stored settings', () => {
    it('never changes the shared defaults when settings change in place', () => {
        loadSettings().types.push('ttf');
        const settings = freshSettings();
        settings.colorNames.push('add');
        settings.order.reverse();
        expect(defaultSettings.types).toEqual(['woff2', 'woff']);
        expect(defaultSettings.colorNames).toEqual([]);
        expect(defaultSettings.order).toEqual(['eot', 'woff2', 'woff', 'ttf', 'svg']);
    });

    it('restores saved settings over the defaults', () => {
        stubStorage();
        saveSettings({ ...freshSettings(), fontName: 'saved', types: ['ttf'] });
        expect(loadSettings()).toEqual({ ...defaultSettings, fontName: 'saved', types: ['ttf'] });
    });

    it('falls back to the defaults when storage is unavailable or damaged', () => {
        expect(loadSettings()).toEqual(defaultSettings);
        expect(() => saveSettings(freshSettings())).not.toThrow();
        stubStorage().set('vite-svg-2-webfont:playground-settings', '{');
        expect(loadSettings()).toEqual(defaultSettings);
    });
});
