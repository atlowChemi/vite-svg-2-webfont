import { icons, iconBaseSelector, iconClassPrefix, type IconName } from 'virtual:test-icons';

const names: readonly ['add'] = icons;
const selector: '.icon' = iconBaseSelector;
const prefix: 'icon-' = iconClassPrefix;
const name: IconName = 'add';

// @ts-expect-error invalid icon names must be rejected
const invalid: IconName = 'missing';
// @ts-expect-error tuple is readonly
icons.push('add');
