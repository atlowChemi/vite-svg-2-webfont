import type { FC } from 'react';
import styles from './Colophon.module.css';

/** Where the name comes from. */
export const ColophonText: FC = () => (
    <>
        <p className={styles.paragraph}>
            <b className={styles.term}>The name.</b> In French, <span className={styles.fonte}>fonte</span> is a casting: metal melted and poured into a mold. The same word means
            melting, as in <i>la fonte des neiges</i>, the snowmelt, and cast iron itself.
        </p>
        <p className={styles.paragraph}>
            <b className={styles.term}>The type.</b> Founders cast letters this way for five centuries. A set of type in one size and style was <i>une fonte</i>, and English took
            its <i>font</i> from it.
        </p>
        <p className={styles.paragraph}>
            <b className={styles.term}>The work.</b> <span className={styles.fonte}>fonte</span> does the same with vectors. Your SVGs are melted down to outlines and cast into
            glyphs, by an engine written in Rust.
        </p>
    </>
);
