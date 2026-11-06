import type { FC } from 'react';
import { toolLink, type PageNav } from '../../site/nav.ts';
import { site, tools } from '../../site/site.ts';
import { SiteIcon } from '../SiteIcon.tsx';
import { Wordmark } from '../Wordmark.tsx';
import styles from './TopBar.module.css';

/** The site's top bar: the wordmark, the tool drawers (a sheet on phones), the Playground, search, GitHub and the theme. */
export const TopBar: FC<{ nav: PageNav }> = ({ nav }) => {
    const pageCount = nav.groups.reduce((n, g) => n + g.items.length, 0);
    return (
        <header className={styles.top} data-top-bar="">
            {/* The landing page docks its own wordmark here; `data-top-wordmark` is what it looks for. */}
            <a className={styles.wordmark} href="/" aria-label="fonte home" data-top-wordmark="">
                <Wordmark />
            </a>
            <button className={styles.toolButton} type="button" aria-haspopup="dialog" aria-expanded="false" data-sheet="tools-sheet">
                <span className={styles.plate}>{nav.tool?.plate ?? 'Start here'}</span>
                <strong>{nav.tool?.name ?? 'Guide'}</strong>
                {pageCount > 0 && <span className={styles.cue}>{pageCount} pages</span>}
                <SiteIcon name="chevron-down" className={styles.caret} />
            </button>
            <nav className={styles.drawers} aria-label="Tools">
                <a className={styles.drawer} href="/guide/" aria-current={nav.isGuide ? 'true' : undefined}>
                    <span className={styles.plate}>Start here</span>
                    <strong>Guide</strong>
                </a>
                {tools.map(t => (
                    <a key={t.key} className={styles.drawer} href={toolLink(t.key, nav.route)} aria-current={t.key === nav.section ? 'true' : undefined}>
                        <span className={styles.plate}>{t.plate}</span>
                        <strong>{t.name}</strong>
                    </a>
                ))}
            </nav>
            <a className={styles.play} href="/playground" aria-label="Playground: generate a font in your browser">
                <SiteIcon name="play" className={styles.mark} />
                <span className={styles.label} aria-hidden="true">
                    <span className={styles.plate}>Try it</span>
                    <strong>Playground</strong>
                </span>
            </a>
            <button type="button" className={styles.search} aria-label="Search" aria-keyshortcuts="/ Control+K Meta+K" data-search="">
                <SiteIcon name="search" />
                <span className={styles.placeholder}>Search</span>
                <kbd>⌘K</kbd>
            </button>
            <a className={styles.icon} href={site.github} target="_blank" rel="noreferrer" aria-label="fonte on GitHub">
                <SiteIcon name="github" />
            </a>
            <button className={styles.icon} type="button" aria-label="Switch theme" title="Switch theme" data-theme-toggle="">
                <SiteIcon name="theme" />
            </button>
        </header>
    );
};
