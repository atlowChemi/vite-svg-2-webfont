import { useEffect, useState } from 'react';
import { defaultSettings, type PlaygroundSettings } from './options.ts';

const SETTINGS_KEY = 'vite-svg-2-webfont:playground-settings';

/** A deep copy of the defaults, so no settings object ever shares the defaults' arrays. */
export function freshSettings(): PlaygroundSettings {
    return structuredClone(defaultSettings);
}

/** The viewer's saved settings over the defaults, or the defaults when nothing is saved or storage is unavailable. */
export function loadSettings(): PlaygroundSettings {
    try {
        const saved = localStorage.getItem(SETTINGS_KEY);
        return { ...freshSettings(), ...(saved && (JSON.parse(saved) as Partial<PlaygroundSettings>)) };
    } catch {
        return freshSettings();
    }
}

export function saveSettings(settings: PlaygroundSettings): void {
    try {
        localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    } catch {
        // Storage can be unavailable (private mode, blocked site data); settings then last for this visit only.
    }
}

/** Playground settings, remembered per viewer. Uploaded SVGs are never stored. */
export function useStoredSettings() {
    const [settings, setSettings] = useState(loadSettings);
    useEffect(() => saveSettings(settings), [settings]);
    return {
        settings,
        update: (patch: Partial<PlaygroundSettings>) => setSettings(current => ({ ...current, ...patch })),
        reset: () => setSettings(freshSettings()),
    };
}
