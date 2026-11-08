/** The most tabs a group can have: `codeTabs.module.css` hides a panel by its position, with a rule for each up to this. */
export const maxTabs = 12;

/** Throws when a group has more tabs than the stylesheet can switch between. */
export function checkTabCount(count: number, where: string): void {
    if (count > maxTabs) throw new Error(`${where} has ${count} tabs; code tabs switch between at most ${maxTabs} (see codeTabs.module.css).`);
}
