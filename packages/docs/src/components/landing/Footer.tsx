import type { FC } from 'react';
import { site } from '../../site/site.ts';
import styles from './Footer.module.css';

/** The page's foot, at its coldest point. */
export const Footer: FC = () => (
    <footer className={styles.footer}>
        <span className={styles.name}>{site.name}</span>
        <a href="/guide/">Docs</a>
        <a href={site.github}>GitHub</a>
        <a href="/rust/changelog">Changelog</a>
        <span className={styles.license}>MIT license · © Chemi Atlow</span>
    </footer>
);
