export const iconBaseSelector = '.icon' as const;
export const iconClassPrefix = 'icon-' as const;
export const icons = [
    'add',
    'arrow-right',
    'bell',
    'check',
    'close',
    'download',
    'flag',
    'heart',
    'home',
    'logo',
    'search',
    'spinner',
    'star',
    'trash',
    'user',
] as const;

export type IconName = (typeof icons)[number];
