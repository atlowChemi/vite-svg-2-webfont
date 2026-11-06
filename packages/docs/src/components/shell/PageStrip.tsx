import type { FC } from 'react';
import { no, type PageNav } from '../../site/nav.ts';
import { normalize } from '../../site/site.ts';
import { cx } from '../cx.ts';
import styles from './PageStrip.module.css';

/** The current group's pages as tabs; the guide adds a switch between its two shelves. Hidden on phones. */
export const PageStrip: FC<{ nav: PageNav }> = ({ nav }) => {
    const { group, groups, isGuide, numbered, route } = nav;
    return (
        <nav className={styles.strip} data-page-strip="" aria-label={`${group?.text ?? ''} pages`}>
            {isGuide && (
                <span className={styles.shelves} role="group" aria-label="Guide shelves">
                    {groups.map(g => (
                        <a key={g.text} className={styles.shelf} href={g.items[0]!.link} aria-current={g === group ? 'true' : undefined}>
                            {g.text}
                        </a>
                    ))}
                </span>
            )}
            {group?.items.map((item, i) => (
                <a key={item.link} className={styles.page} href={item.link} aria-current={normalize(item.link) === route ? 'page' : undefined}>
                    {numbered && <span className={styles.number}>{no(i)}</span>}
                    {item.text}
                </a>
            ))}
            {!isGuide && (
                <a className={cx(styles.page, styles.guide)} href="/guide/">
                    Guide &amp; features →
                </a>
            )}
        </nav>
    );
};
