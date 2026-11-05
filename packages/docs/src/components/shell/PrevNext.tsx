import type { FC } from 'react';
import { no, type PageNav } from '../../site/nav.ts';
import { cx } from '../cx.ts';
import styles from './PrevNext.module.css';

/** The previous and next pages in the current group. */
export const PrevNext: FC<{ nav: PageNav }> = ({ nav }) => {
    const { prev, next, numbered, index } = nav;
    return (
        <nav className={styles.next} aria-label="Previous and next pages">
            {prev ? (
                <a className={styles.link} href={prev.link}>
                    <span className={styles.cue}>← {numbered ? `No. ${no(index - 1)}` : 'Previous'}</span>
                    <strong>{prev.text}</strong>
                </a>
            ) : (
                <span />
            )}
            {next && (
                <a className={cx(styles.link, styles.right)} href={next.link}>
                    <span className={styles.cue}>{numbered ? `No. ${no(index + 1)}` : 'Next'} →</span>
                    <strong>{next.text}</strong>
                </a>
            )}
        </nav>
    );
};
