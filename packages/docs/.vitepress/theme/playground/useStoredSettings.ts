import { ref, watch } from 'vue';
import { defaultSettings, type PlaygroundSettings } from './options';

const SETTINGS_KEY = 'vite-svg-2-webfont:playground-settings';

function loadSettings(): PlaygroundSettings {
    try {
        const saved = localStorage.getItem(SETTINGS_KEY);
        return { ...structuredClone(defaultSettings), ...(saved && (JSON.parse(saved) as Partial<PlaygroundSettings>)) };
    } catch {
        return structuredClone(defaultSettings);
    }
}

/** Playground settings, remembered per viewer. Uploaded SVGs are never stored. */
export function useStoredSettings() {
    const settings = ref<PlaygroundSettings>(loadSettings());
    watch(
        settings,
        value => {
            try {
                localStorage.setItem(SETTINGS_KEY, JSON.stringify(value));
            } catch {
                // Storage can be unavailable (private mode, blocked site data); settings then last for this visit only.
            }
        },
        { deep: true },
    );
    // A deep copy: settings change in place, which must never reach the shared defaults' arrays.
    const reset = () => (settings.value = structuredClone(defaultSettings));
    return { settings, reset };
}
