import { normalize, sections, tools, type NavGroup, type NavItem, type Tool } from './site.ts';

/** Where a page sits in the navigation: its section, tool, page group, and neighbours. */
export interface PageNav {
    route: string;
    section: string;
    tool?: Tool;
    isGuide: boolean;
    /** The section's own page groups: a tool's pages, or the guide's two shelves. */
    groups: NavGroup[];
    /** The group this page belongs to, listed in the page strip. */
    group?: NavGroup;
    index: number;
    prev?: NavItem;
    next?: NavItem;
    /** Only the guide is read in order, so only its pages carry a number; tool pages and features are looked up. */
    numbered: boolean;
}

export function pageNav(route: string): PageNav {
    const current = normalize(route);
    const section = current.split('/', 2)[1] ?? '';
    const groups = sections[section] ?? [];
    const group = groups.find(g => g.items.some(i => normalize(i.link) === current)) ?? groups[0];
    const index = group?.items.findIndex(i => normalize(i.link) === current) ?? -1;
    return {
        route: current,
        section,
        tool: tools.find(t => t.key === section),
        isGuide: section === 'guide',
        groups,
        group,
        index,
        prev: group && index > 0 ? group.items[index - 1] : undefined,
        next: group && index >= 0 && index < group.items.length - 1 ? group.items[index + 1] : undefined,
        numbered: section === 'guide' && group === groups[0],
    };
}

/** Switching tools keeps your place: Node's Configuration → Vite's Configuration, when it has one. */
export function toolLink(key: string, route: string): string {
    const pageType = normalize(route).split('/', 3)[2];
    const same = pageType && sections[key]?.[0]?.items.find(i => normalize(i.link) === `/${key}/${pageType}`);
    return same ? same.link : `/${key}/`;
}

/** `01`, `02`, … for numbered pages and sections. */
export const no = (i: number) => String(i + 1).padStart(2, '0');
