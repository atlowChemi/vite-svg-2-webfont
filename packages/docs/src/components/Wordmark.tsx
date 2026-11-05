import { useId, type FC } from 'react';
import { letters, moltenSurface, tPath, tTransform, viewBox } from '../site/wordmark.ts';
import { cx } from './cx.ts';
import styles from './Wordmark.module.css';

interface Props {
    /** Replaces the still molten surface (in the T's 100×86 box), e.g. with a frame of the landing page's pour. */
    surface?: string;
    className?: string;
}

/**
 * The fonte wordmark, inline rather than an `<img>` so the letters and the mold follow the site theme instead of the OS
 * color scheme. The landing page pours into it, finding the parts by `data-part`.
 */
export const Wordmark: FC<Props> = ({ surface = moltenSurface, className }) => {
    const clip = useId();
    return (
        <svg className={cx(styles.wordmark, className)} viewBox={viewBox} role="img" aria-label="fonte">
            <path className={styles.ink} d={letters} />
            <g transform={tTransform}>
                <clipPath id={clip}>
                    <path d={tPath} />
                </clipPath>
                <g clipPath={`url(#${clip})`}>
                    <path className={styles.molten} data-part="molten" d={`${surface} L100 100 L0 100 Z`} />
                    <path className={styles.surface} data-part="surface" d={surface} />
                </g>
                <path className={styles.mold} data-part="mold" d={tPath} />
            </g>
        </svg>
    );
};
