import type { FC } from 'react';
import { siteIcon, type SiteIcon as Name } from '../site/siteIcons.ts';
import { cx } from './cx.ts';

/** A glyph from the site's icon font; decoration, so hidden from assistive technology. */
export const SiteIcon: FC<{ name: Name; className?: string }> = ({ name, className }) => <i className={cx(siteIcon(name), className)} aria-hidden="true" />;
