import type { FC } from 'react';
import { repo } from '../../site/site.ts';
import styles from './SiteFooter.module.css';

const changelogs = [
    { text: 'Vite Plugin', link: '/vite/changelog' },
    { text: 'Node.js', link: '/node/changelog' },
    { text: 'Browser', link: '/browser/changelog' },
    { text: 'Rust Engine & CLI', link: '/rust/changelog' },
];
const registries = [
    { text: 'crates.io', link: 'https://crates.io/crates/webfont-generator' },
    { text: 'npm', link: 'https://www.npmjs.com/package/vite-svg-2-webfont' },
    { text: 'GitHub', link: repo },
];

/** The foot of the full-width pages: changelogs, where the packages live, and the license. */
export const SiteFooter: FC = () => (
    <footer className={styles.footer}>
        <nav className={styles.links} aria-label="Changelogs">
            <span className={styles.plate}>Changelogs</span>
            {changelogs.map(c => (
                <a key={c.link} href={c.link}>
                    {c.text}
                </a>
            ))}
        </nav>
        <nav className={styles.links} aria-label="Packages">
            {registries.map(r => (
                <a key={r.link} href={r.link} target="_blank" rel="noreferrer">
                    {r.text}
                </a>
            ))}
        </nav>
        <p className={styles.license}>Released under the MIT License. Copyright © Chemi Atlow</p>
    </footer>
);
