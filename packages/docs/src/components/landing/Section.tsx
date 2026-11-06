import type { FC, ReactNode } from 'react';
import { cx } from '../cx.ts';
import { degrees } from './pour.ts';
import styles from './Section.module.css';

interface SectionProps {
    num: string;
    name: string;
    id?: string;
    className?: string;
    /** The rail's own adjustments, e.g. the hero's taller rail. */
    railClassName?: string;
    /** A rule above the section; the hero, first on the page, has none. */
    ruled?: boolean;
    children: ReactNode;
}

/**
 * One numbered section of the specimen sheet: a sticky rail with the section's number, name and the page's temperature
 * (the landing script updates it as the page cools), then the content.
 */
export const Section: FC<SectionProps> = ({ num, name, id, className, railClassName, ruled = true, children }) => (
    <section id={id} className={cx(styles.section, ruled && styles.ruled, className)}>
        <div className={cx(styles.rail, railClassName)}>
            <span className={styles.no}>No.</span>
            <span className={styles.num}>{num}</span>
            <span className={styles.name}>{name}</span>
            <span className={styles.deg}>{degrees(1)}</span>
        </div>
        {children}
    </section>
);

interface SectionHeadingProps {
    title: string;
    lede: string;
    illustration?: string;
}

/**
 * A section's title and lede, with an illustration beside them when there is one. Illustrations load lazily: they sit
 * below the hero, and React would otherwise emit a preload link for each.
 */
export const SectionHeading: FC<SectionHeadingProps> = ({ title, lede, illustration }) => {
    const text = (
        <>
            <h2 className={styles.title}>{title}</h2>
            <p className={styles.lede}>{lede}</p>
        </>
    );
    if (!illustration) return text;
    return (
        <div className={styles.heading}>
            <img className={styles.illustration} src={illustration} alt="" loading="lazy" />
            <div>{text}</div>
        </div>
    );
};
