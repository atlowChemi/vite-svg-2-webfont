import { Fragment, type FC } from 'react';
import type { Heading } from '../../build/markdown.ts';
import { no, toolLink, type PageNav } from '../../site/nav.ts';
import { normalize, site, tools } from '../../site/site.ts';
import { cx } from '../cx.ts';
import { filterable } from './Rail.tsx';
import styles from './Sheets.module.css';

// Phone sheets. Each is a native modal dialog: the browser moves focus in, keeps it there, and closes it on Escape.

/** The guide and every tool; the current one lists its pages underneath, in place of the page strip. */
export const ToolsSheet: FC<{ nav: PageNav }> = ({ nav }) => {
    const entries = [
        { key: 'guide', plate: 'Start here', name: 'Guide', pkg: 'Concepts and features', link: '/guide/' },
        ...tools.map(t => ({ key: t.key, plate: t.plate, name: t.name, pkg: t.package, link: toolLink(t.key, nav.route) })),
    ];
    return (
        <dialog id="tools-sheet" className={styles.sheet} aria-labelledby="tools-sheet-title" tabIndex={-1}>
            <div id="tools-sheet-title" className={styles.title}>
                Tools
            </div>
            {entries.map(entry => (
                <Fragment key={entry.key}>
                    <a className={styles.tool} href={entry.link} aria-current={entry.key === nav.section ? 'true' : undefined}>
                        <span className={styles.plate}>{entry.plate}</span>
                        <strong>{entry.name}</strong>
                        <span className={styles.package}>{entry.pkg}</span>
                    </a>
                    {entry.key === nav.section && nav.groups.length > 0 && (
                        <div className={styles.pages}>
                            {nav.groups.map(g => (
                                <Fragment key={g.text}>
                                    {nav.groups.length > 1 && <div className={styles.shelf}>{g.text}</div>}
                                    {g.items.map(item => (
                                        <a key={item.link} className={styles.page} href={item.link} aria-current={normalize(item.link) === nav.route ? 'page' : undefined}>
                                            {item.text}
                                        </a>
                                    ))}
                                </Fragment>
                            ))}
                        </div>
                    )}
                </Fragment>
            ))}
            <a className={cx(styles.tool, styles.play)} href="/playground">
                <span className={styles.plate}>Try it</span>
                <strong>Playground</strong>
                <span className={styles.package}>Generate a font in your browser</span>
            </a>
            <div className={styles.links}>
                <a href={site.github} target="_blank" rel="noreferrer">
                    <span className={styles.plate}>Source</span>GitHub ↗
                </a>
                {nav.tool && (
                    <a href={nav.tool.registry} target="_blank" rel="noreferrer">
                        <span className={styles.plate}>Package</span>
                        {nav.tool.registryName} ↗
                    </a>
                )}
                <button type="button" data-theme-toggle="">
                    <span className={styles.plate}>Theme</span>
                    <span data-theme-label="">Switch</span>
                </button>
            </div>
        </dialog>
    );
};

/** Phones: the rail's index becomes a pill that names the current section; it opens the sections sheet. */
export const SectionPill: FC<{ headings: Heading[] }> = ({ headings }) => (
    <button className={styles.pill} type="button" aria-haspopup="dialog" aria-expanded="false" data-sheet="sections-sheet" data-pill="">
        <span className={styles.pillNumber} data-pill-number="">
            01
        </span>
        <span data-pill-text="">{headings[0]!.title}</span>
        <span className={styles.pillCaret} aria-hidden="true">
            ▴
        </span>
    </button>
);

export const SectionsSheet: FC<{ name: string; headings: Heading[] }> = ({ name, headings }) => (
    <dialog id="sections-sheet" className={styles.sheet} aria-labelledby="sections-sheet-title" tabIndex={-1}>
        <div id="sections-sheet-title" className={styles.title}>
            On this page · {name}
        </div>
        {filterable(headings) && <input className={styles.filter} type="search" placeholder="Filter this page" aria-label="Filter this page's sections" data-filter="" />}
        {headings.map((s, i) => (
            <Fragment key={s.slug}>
                {/* The page opens at its top, in the first section; the page script follows the reader from there. */}
                <a className={styles.section} href={`#${s.slug}`} data-section="" aria-current={i === 0 ? 'location' : undefined}>
                    <span className={styles.number}>{no(i)}</span>
                    {s.title}
                </a>
                {s.children.map(c => (
                    <a key={c.slug} className={cx(styles.section, styles.child)} href={`#${c.slug}`} data-section="child">
                        {c.title}
                    </a>
                ))}
            </Fragment>
        ))}
    </dialog>
);
