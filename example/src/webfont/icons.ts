export const iconBaseSelector = '.exIcon' as const;
export const iconClassPrefix = 'icon-' as const;
export const icons = [
    'calendar',
    'add',
] as const;

export type IconOption = typeof icons[number];
