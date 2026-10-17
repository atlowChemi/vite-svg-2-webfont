import { ref, type Ref } from 'vue';
import { formatCodepoint, nextCodepoint, parseCodepoint } from './codepoints';
import type { SvgSourceFile } from './files';
import { iconName, isValidName, uniqueName } from './names';
import type { PlaygroundIcon, PlaygroundSettings } from './options';

/** A rejected edit leaves the icon unchanged, so the font keeps building with its last valid value. */
function withFallback(problem: string, previous: string): string {
    return `${problem}. Falling back to ${previous}.`;
}

export type IconErrors = Record<string, { name?: string; codepoint?: string }>;

/** The playground's icons and the rules for adding, renaming, re-numbering and removing them. */
export function useIconList(settings: Ref<PlaygroundSettings>) {
    const icons = ref<PlaygroundIcon[]>([]);
    const errors = ref<IconErrors>({});

    function setError(id: string, field: 'name' | 'codepoint', message?: string) {
        (errors.value[id] ??= {})[field] = message;
    }

    /** Add SVGs; a file whose name matches an existing icon replaces that icon's drawing and keeps its codepoint. */
    function addSources(sources: SvgSourceFile[]) {
        const names = new Set(icons.value.map(icon => icon.name));
        const used = new Set(icons.value.map(icon => icon.codepoint));
        for (const { fileName, svg } of sources) {
            const name = iconName(fileName);
            const existing = icons.value.find(icon => icon.name === name);
            if (existing) {
                existing.svg = svg;
                continue;
            }
            const icon = { id: crypto.randomUUID(), name: uniqueName(name, names), svg, codepoint: nextCodepoint(settings.value.startCodepoint, used) };
            names.add(icon.name);
            used.add(icon.codepoint);
            icons.value.push(icon);
        }
    }

    function rename(icon: PlaygroundIcon, name: string) {
        const taken = icons.value.some(other => other.id !== icon.id && other.name === name);
        const problem = !isValidName(name) ? 'Use letters, numbers, - and _ only' : taken ? 'Another icon has this name' : undefined;
        setError(icon.id, 'name', problem && withFallback(problem, icon.name));
        if (problem) return;
        const { colorNames } = settings.value;
        const selected = colorNames.indexOf(icon.name);
        if (selected !== -1) colorNames[selected] = name;
        icon.name = name;
    }

    function setCodepoint(icon: PlaygroundIcon, text: string) {
        const codepoint = parseCodepoint(text);
        const taken = codepoint !== undefined && icons.value.some(other => other.id !== icon.id && other.codepoint === codepoint);
        const problem = codepoint === undefined ? 'Enter a hexadecimal codepoint, such as F101' : taken ? 'Another icon uses this codepoint' : undefined;
        setError(icon.id, 'codepoint', problem && withFallback(problem, formatCodepoint(icon.codepoint)));
        if (problem || codepoint === undefined) return;
        icon.codepoint = codepoint;
    }

    function remove(icon: PlaygroundIcon) {
        icons.value.splice(
            icons.value.findIndex(item => item.id === icon.id),
            1,
        );
        const { colorNames } = settings.value;
        const selected = colorNames.indexOf(icon.name);
        if (selected !== -1) colorNames.splice(selected, 1);
    }

    function removeAll() {
        icons.value = [];
        errors.value = {};
    }

    /** Number every icon again from the start codepoint, in list order. */
    function reassign() {
        const used = new Set<number>();
        for (const icon of icons.value) {
            icon.codepoint = nextCodepoint(settings.value.startCodepoint, used);
            used.add(icon.codepoint);
        }
        errors.value = {};
    }

    return { icons, errors, addSources, rename, setCodepoint, remove, removeAll, reassign };
}
