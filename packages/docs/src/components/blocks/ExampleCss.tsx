import type { FC } from 'react';
import type { Highlighter } from 'shiki';
import { highlight } from '../../build/highlight.ts';
import styles from './ExampleCss.module.css';

interface Props {
    /** The CSS the plugin generated for one example. */
    css: string;
    /** Lines containing any of these snippets are highlighted, matching the config tab's highlights. */
    highlight?: string[];
    /** A highlighter with CSS loaded. */
    shiki: Highlighter;
}

/** An example's generated CSS, highlighted like every other code block. */
export const ExampleCss: FC<Props> = ({ css, highlight: snippets = [], shiki }) => {
    const lines = new Set(css.split('\n').flatMap((line, i) => (snippets.some(snippet => line.includes(snippet)) ? [i + 1] : [])));
    return <div className={styles.css} dangerouslySetInnerHTML={{ __html: highlight(shiki, css, 'css', { lines }) }} />;
};
