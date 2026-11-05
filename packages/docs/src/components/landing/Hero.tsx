import type { FC } from 'react';
import { codepointLabel, trayIcons, type SiteIcon as IconName } from '../../site/siteIcons.ts';
import { cx } from '../cx.ts';
import { SiteIcon } from '../SiteIcon.tsx';
import { Wordmark } from '../Wordmark.tsx';
import { emptySurface } from './pour.ts';
import { Section } from './Section.tsx';
import styles from './Hero.module.css';

/**
 * The first section: a tray of icons above the empty mold, which the landing script pours into the T as the reader
 * scrolls through the pinned stage, revealing the cast font at the end.
 */
export const Hero: FC = () => (
    <Section num="01" name="Casting" className={styles.hero} railClassName={styles.rail} ruled={false}>
        <div className={styles.stage}>
            <div className={styles.tray} aria-hidden="true">
                {trayIcons.map(name => (
                    <span key={name} className={styles.trayItem}>
                        <SiteIcon name={name} className={styles.icon} />
                    </span>
                ))}
            </div>
            <div className={styles.placeholder}>
                <Wordmark surface={emptySurface} className={styles.wordmark} />
            </div>
            <h1 className={styles.tag}>Melted SVGs, cast into fonts</h1>
            <p className={styles.sub}>A Rust engine for icon fonts, for the CLI, Node, the browser, and Vite.</p>
            <div className={styles.hint} aria-hidden="true">
                <span>scroll to cast</span>
                <SiteIcon name="arrow-down" className={styles.icon} />
            </div>
            <CastResult fileName="my-icons.woff2" icons={trayIcons} />
        </div>
    </Section>
);

interface CastResultProps {
    fileName: string;
    icons: readonly IconName[];
}

/** The font the pour casts: its file, a cell per glyph with its codepoint, and where to go next. */
export const CastResult: FC<CastResultProps> = ({ fileName, icons }) => (
    <div className={styles.result}>
        <div className={styles.resultHead}>
            <span>{fileName}</span>
            <span>{icons.length} glyphs</span>
        </div>
        <div className={styles.cells}>
            {icons.map(name => (
                <div key={name} className={styles.cell}>
                    <SiteIcon name={name} className={styles.icon} />
                    <code>U+{codepointLabel(name)}</code>
                </div>
            ))}
        </div>
        <div className={styles.actions}>
            <a className={cx(styles.button, styles.brand)} href="/guide/">
                Get started
            </a>
            <a className={styles.button} href="/playground">
                Try the playground
            </a>
        </div>
    </div>
);
