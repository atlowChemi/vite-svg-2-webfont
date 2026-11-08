import type { FontType } from '@atlowchemi/webfont-browser-generator';
import { selectedTypes, type PlaygroundSettings } from './options.ts';

/** The settings an empty field leaves to the engine. */
export type NumberSetting = 'fontHeight' | 'ascent' | 'descent' | 'round' | 'ttfTimestamp';

/** A number field's value: undefined when empty or not a finite number. */
export function parseOptionalNumber(text: string): number | undefined {
    const value = text.trim() === '' ? undefined : Number(text);
    return value !== undefined && Number.isFinite(value) ? value : undefined;
}

/** `list` with `item` added (at the end) or removed; the same list when nothing changes. */
export function toggled<T>(list: readonly T[], item: T, enabled: boolean): T[] {
    const present = list.includes(item);
    if (enabled === present) return list as T[];
    return enabled ? [...list, item] : list.filter(entry => entry !== item);
}

/**
 * The format order with a selected format moved one step among the selected ones, swapping past any unselected
 * formats between them; undefined when it is already first or last.
 */
export function movedFormat(settings: PlaygroundSettings, type: FontType, step: -1 | 1): FontType[] | undefined {
    const visible = selectedTypes(settings);
    const neighbour = visible[visible.indexOf(type) + step];
    if (!neighbour) return undefined;
    const order = [...settings.order];
    const from = order.indexOf(type);
    const to = order.indexOf(neighbour);
    [order[from], order[to]] = [order[to]!, order[from]!];
    return order;
}
