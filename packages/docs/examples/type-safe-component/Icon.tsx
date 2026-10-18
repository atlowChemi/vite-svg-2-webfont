import { iconBaseSelector, iconClassPrefix, type IconName } from './icons';

export function Icon({ name }: { name: IconName }) {
    return <i className={`${iconBaseSelector.slice(1)} ${iconClassPrefix}${name}`} aria-hidden="true" />;
}
