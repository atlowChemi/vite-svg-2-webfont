export const iconBaseSelector = '.exIcon' as const;
export const iconClassPrefix = 'icon-' as const;
export const icons = [
    'add',
    'calendar',
] as const;

export type IconOption = typeof icons[number];
