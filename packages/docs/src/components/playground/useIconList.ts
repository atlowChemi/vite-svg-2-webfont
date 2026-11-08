import { useState } from 'react';
import type { SvgSourceFile } from './files.ts';
import { addSources, emptyIconList, reassignCodepoints, removeIcon, renameIcon, setIconCodepoint } from './iconList.ts';
import type { PlaygroundIcon, PlaygroundSettings } from './options.ts';

/** The playground's icons and the rules for adding, renaming, re-numbering and removing them (see `iconList.ts`). */
export function useIconList(settings: PlaygroundSettings, updateSettings: (patch: Partial<PlaygroundSettings>) => void) {
    const [list, setList] = useState(emptyIconList);

    function updateColorNames(colorNames: string[]) {
        if (colorNames !== settings.colorNames) updateSettings({ colorNames });
    }

    return {
        icons: list.icons,
        errors: list.errors,
        // Files are read asynchronously, so this applies to whatever the list is by then.
        addSources: (sources: SvgSourceFile[]) => setList(current => addSources(current, sources, settings.startCodepoint)),
        rename: (icon: PlaygroundIcon, name: string) => {
            const next = renameIcon(list, settings.colorNames, icon.id, name);
            setList(next.list);
            updateColorNames(next.colorNames);
        },
        setCodepoint: (icon: PlaygroundIcon, text: string) => setList(setIconCodepoint(list, icon.id, text)),
        remove: (icon: PlaygroundIcon) => {
            const next = removeIcon(list, settings.colorNames, icon.id);
            setList(next.list);
            updateColorNames(next.colorNames);
        },
        removeAll: () => setList(emptyIconList),
        reassign: () => setList(reassignCodepoints(list, settings.startCodepoint)),
    };
}
