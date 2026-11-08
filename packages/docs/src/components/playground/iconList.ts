import { formatCodepoint, nextCodepoint, parseCodepoint } from './codepoints.ts';
import type { SvgSourceFile } from './files.ts';
import { iconName, isValidName, uniqueName } from './names.ts';
import type { PlaygroundIcon } from './options.ts';

export type IconErrors = Record<string, { name?: string; codepoint?: string }>;

/** The playground's icons, in the order they were added, and the last rejected edit of each. */
export type IconList = { icons: PlaygroundIcon[]; errors: IconErrors };

export const emptyIconList: IconList = { icons: [], errors: {} };

/** A rejected edit leaves the icon unchanged, so the font keeps building with its last valid value. */
function withFallback(problem: string, previous: string): string {
    return `${problem}. Falling back to ${previous}.`;
}

function withError(errors: IconErrors, id: string, field: 'name' | 'codepoint', message?: string): IconErrors {
    return { ...errors, [id]: { ...errors[id], [field]: message } };
}

/** Add SVGs; a file whose name matches an existing icon replaces that icon's drawing and keeps its codepoint. */
export function addSources(list: IconList, sources: readonly SvgSourceFile[], startCodepoint: number): IconList {
    if (!sources.length) return list;
    const icons = [...list.icons];
    const names = new Set(icons.map(icon => icon.name));
    const used = new Set(icons.map(icon => icon.codepoint));
    for (const { fileName, svg } of sources) {
        const name = iconName(fileName);
        const existing = icons.findIndex(icon => icon.name === name);
        if (existing !== -1) {
            icons[existing] = { ...icons[existing]!, svg };
            continue;
        }
        const icon = { id: crypto.randomUUID(), name: uniqueName(name, names), svg, codepoint: nextCodepoint(startCodepoint, used) };
        names.add(icon.name);
        used.add(icon.codepoint);
        icons.push(icon);
    }
    return { ...list, icons };
}

/** Rename an icon, carrying its color-glyph selection over; an invalid or taken name is rejected with an error. */
export function renameIcon(list: IconList, colorNames: string[], id: string, name: string): { list: IconList; colorNames: string[] } {
    const icon = list.icons.find(item => item.id === id);
    if (!icon) return { list, colorNames };
    const taken = list.icons.some(other => other.id !== id && other.name === name);
    const problem = !isValidName(name) ? 'Use letters, numbers, - and _ only' : taken ? 'Another icon has this name' : undefined;
    const errors = withError(list.errors, id, 'name', problem && withFallback(problem, icon.name));
    if (problem) return { list: { ...list, errors }, colorNames };
    return {
        list: { icons: list.icons.map(item => (item.id === id ? { ...item, name } : item)), errors },
        colorNames: colorNames.includes(icon.name) ? colorNames.map(selected => (selected === icon.name ? name : selected)) : colorNames,
    };
}

/** Set an icon's codepoint from text such as `F101` or `U+E001`; unparsable or taken codepoints are rejected. */
export function setIconCodepoint(list: IconList, id: string, text: string): IconList {
    const icon = list.icons.find(item => item.id === id);
    if (!icon) return list;
    const codepoint = parseCodepoint(text);
    const taken = codepoint !== undefined && list.icons.some(other => other.id !== id && other.codepoint === codepoint);
    const problem = codepoint === undefined ? 'Enter a hexadecimal codepoint, such as F101' : taken ? 'Another icon uses this codepoint' : undefined;
    const errors = withError(list.errors, id, 'codepoint', problem && withFallback(problem, formatCodepoint(icon.codepoint)));
    if (problem || codepoint === undefined) return { ...list, errors };
    return { icons: list.icons.map(item => (item.id === id ? { ...item, codepoint } : item)), errors };
}

/** Remove an icon, and drop it from the color-glyph selection. */
export function removeIcon(list: IconList, colorNames: string[], id: string): { list: IconList; colorNames: string[] } {
    const icon = list.icons.find(item => item.id === id);
    if (!icon) return { list, colorNames };
    const { [id]: _removed, ...errors } = list.errors;
    return {
        list: { icons: list.icons.filter(item => item.id !== id), errors },
        colorNames: colorNames.includes(icon.name) ? colorNames.filter(selected => selected !== icon.name) : colorNames,
    };
}

/** Number every icon again from the start codepoint, in list order, and clear every error. */
export function reassignCodepoints(list: IconList, startCodepoint: number): IconList {
    const used = new Set<number>();
    const icons = list.icons.map(icon => {
        const codepoint = nextCodepoint(startCodepoint, used);
        used.add(codepoint);
        return { ...icon, codepoint };
    });
    return { icons, errors: {} };
}
