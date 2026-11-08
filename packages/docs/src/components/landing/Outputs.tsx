import type { FC } from 'react';
import { cx } from '../cx.ts';
import styles from './Outputs.module.css';

const formats = ['woff2', 'woff', 'ttf', 'eot', 'svg'];

interface Props {
    snippet: string;
}

/** What a cast produces: the files beside a sample of them (`snippet`, the rendered `outputs` slot of index.md). */
export const Outputs: FC<Props> = ({ snippet }) => (
    <div className={styles.files}>
        <div>
            <div className={styles.file}>
                <span className={styles.ext}>fonts</span>
                <span className={cx(styles.what, styles.chips)}>
                    {formats.map(format => (
                        <span key={format} className={styles.chip}>
                            {format}
                        </span>
                    ))}
                </span>
            </div>
            <OutputFile ext=".css .scss" what="A class for every glyph, or your own template" />
            <OutputFile ext=".html" what="A preview page of the whole set" />
            <OutputFile ext=".ts" what="Icon names and a union type, for type-safe components" />
        </div>
        <div className={styles.snippet} dangerouslySetInnerHTML={{ __html: snippet }} />
    </div>
);

const OutputFile: FC<{ ext: string; what: string }> = ({ ext, what }) => (
    <div className={styles.file}>
        <span className={styles.ext}>{ext}</span>
        <span className={styles.what}>{what}</span>
    </div>
);
