import type { CSSProperties, FC } from 'react';
import { codepointLabel, trayIcons } from '../site/siteIcons.ts';
import { cx } from './cx.ts';
import styles from './NotFound.module.css';
import { SiteIcon } from './SiteIcon.tsx';

// A type case of glyphs cast in the site's icon font, with one empty compartment: the sort for this page. A highlight
// scans the case once and stops on the gap; with reduced motion, the gap is marked straight away. NotFound.module.css
// runs the scan from each cell's `--i`.
const missing = 10;
const cells = Array.from({ length: 18 }, (_, i) => trayIcons[(i * 5) % trayIcons.length]!);

/** The 404 page: what went missing, the way back, and the case with its empty compartment. */
export const NotFound: FC = () => (
    <div className={styles.notFound}>
        <div>
            <h1>
                <span className={styles.code}>404</span>
                <span className={styles.line}>This sort isn’t in the case.</span>
            </h1>
            <p className={styles.lede}>A sort is one piece of cast type. The page you asked for isn’t in this case: it may have moved when the docs were reorganized by tool.</p>
            <p className={cx(styles.mono, styles.where)}>U+E404 · not found</p>
            <div className={styles.actions}>
                <a className={cx(styles.button, styles.brand)} href="/guide/">
                    Go to the guide
                </a>
                <a className={styles.button} href="/">
                    Home
                </a>
            </div>
        </div>
        <div className={styles.case} aria-hidden="true">
            {cells.map((cell, i) =>
                i === missing ? (
                    <div key={i} className={cx(styles.cell, styles.hole)} style={{ '--i': i } as CSSProperties} />
                ) : (
                    // Only the cells before the gap are scanned.
                    <div key={i} className={cx(styles.cell, i < missing && styles.scan)} style={i < missing ? ({ '--i': i } as CSSProperties) : undefined}>
                        <SiteIcon name={cell} className={styles.glyph} />
                        <span className={styles.mono}>{codepointLabel(cell)}</span>
                    </div>
                ),
            )}
        </div>
    </div>
);
