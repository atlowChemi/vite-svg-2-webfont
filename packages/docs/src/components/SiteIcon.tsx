import { siteIcon, type SiteIcon as Name } from '../site/siteIcons.ts';

/** A glyph from the site's icon font; decoration, so hidden from assistive technology. */
export function SiteIcon({ name, className }: { name: Name; className?: string }) {
    return <i className={className ? `${siteIcon(name)} ${className}` : siteIcon(name)} aria-hidden="true" />;
}
