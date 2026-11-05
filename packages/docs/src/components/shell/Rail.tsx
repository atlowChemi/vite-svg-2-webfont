import type { FC } from 'react';
import type { Heading } from '../../build/markdown.ts';
import { no, type PageNav } from '../../site/nav.ts';
import { cx } from '../cx.ts';
import styles from './Rail.module.css';

/** Ten entries or more get a filter. */
export const filterable = (headings: Heading[]) => headings.reduce((n, s) => n + s.children.length, 0) >= 10;

/**
 * The rail's index: the page's h2 sections, each with its h3 entries (options, functions). Every entry is in the markup;
 * the page script shows only the current section's entries and follows the reader.
 */
const SectionIndex: FC<{ headings: Heading[] }> = ({ headings }) => (
    <nav className={styles.index} aria-label="On this page" data-index="">
        {filterable(headings) && <input className={styles.filter} type="search" placeholder="Filter this page" aria-label="Filter this page's sections" data-filter="" />}
        <ol>
            {headings.map((s, i) => (
                <li key={s.slug}>
                    <a href={`#${s.slug}`}>
                        <span className={styles.sectionNumber}>{no(i)}</span>
                        {s.title}
                    </a>
                    {s.children.length > 0 && (
                        <ol hidden>
                            {s.children.map(c => (
                                <li key={c.slug}>
                                    <a href={`#${c.slug}`}>{c.title}</a>
                                </li>
                            ))}
                        </ol>
                    )}
                </li>
            ))}
        </ol>
        <p className={styles.empty} hidden data-no-match="">
            Nothing matches.
        </p>
    </nav>
);

interface Props {
    nav: PageNav;
    name: string;
    headings: Heading[];
}

/** The page's plate: its number or group, its name, its package; then the section index. */
export const Rail: FC<Props> = ({ nav, name, headings }) => {
    const { tool, numbered, group, index } = nav;
    return (
        <aside className={styles.rail} data-rail="">
            {numbered ? (
                <>
                    <div className={styles.dim}>No.</div>
                    <div className={styles.number}>{no(index)}</div>
                </>
            ) : (
                <div className={styles.dim}>{tool ? tool.plate : group?.text}</div>
            )}
            <div className={styles.name}>{name}</div>
            {tool ? (
                <a className={cx(styles.dim, styles.tool)} href={tool.registry} target="_blank" rel="noreferrer">
                    {tool.name} · {tool.package} ↗
                </a>
            ) : (
                numbered && <div className={cx(styles.dim, styles.tool)}>{group?.text}</div>
            )}
            {headings.length > 0 && <SectionIndex headings={headings} />}
        </aside>
    );
};
