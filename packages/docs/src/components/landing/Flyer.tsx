import type { FC } from 'react';
import { Wordmark } from '../Wordmark.tsx';
import styles from './Flyer.module.css';
import { emptySurface } from './pour.ts';

/**
 * The hero wordmark that pours and then docks into the top bar. It is fixed and hidden until the landing script has
 * measured where it starts and where it lands; the copy in the hero's placeholder keeps its place until then.
 */
export const Flyer: FC = () => (
    <div className={styles.flyer} aria-hidden="true">
        <Wordmark surface={emptySurface} className={styles.wordmark} />
    </div>
);
