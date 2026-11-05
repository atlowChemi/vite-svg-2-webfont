import type { CSSProperties, FC } from 'react';
import { svgAuthoringFonts } from '../../site/svgAuthoringCases.ts';
import { cx } from '../cx.ts';
import styles from './SvgAuthoringCompare.module.css';

const sources = import.meta.glob<string>('../../../svg-authoring-fixtures/**/*.svg', { query: '?raw', import: 'default', eager: true });

interface Props {
    /** Fonts to compare, in display order. */
    cases: { id: string; label: string }[];
    /** Render each case's icons inline between these two words, to show alignment against text. */
    text?: [string, string];
    /** Render these raw codepoints instead of class names. */
    codepoints?: number[];
    /** Glyph size in pixels; larger sizes reveal outline detail. */
    size?: number;
}

/** Class names exactly as an app would write them, mapped onto this case's font, whose CSS is global. */
const classes = (id: string, name: string) => `sa-${id} sa-${id}-${name}`;
const displayName = (name: string) => name.replace(/^[a-z]-/, '');
const hex = (codepoint: number) => `U+${codepoint.toString(16).toUpperCase()}`;

function iconsOf(id: string) {
    const dir = svgAuthoringFonts.find(font => font.id === id)?.dir ?? id;
    return Object.entries(sources)
        .filter(([path]) => path.split('/').at(-2) === dir)
        .map(([path, svg]) => ({
            name: path
                .split('/')
                .at(-1)!
                .replace(/\.svg$/, ''),
            svg,
        }))
        .toSorted((a, b) => a.name.localeCompare(b.name));
}

/** Source SVGs beside the glyphs fonte cast from them, one column per font case, each case its own live font. */
export const SvgAuthoringCompare: FC<Props> = ({ cases, text, codepoints, size = 48 }) => (
    <div className={cx(styles.compare, text && styles.stacked)} style={{ '--glyph-size': `${size}px` } as CSSProperties}>
        {cases.map(({ id, label }) => {
            const icons = iconsOf(id);
            return (
                <section key={id} className={styles.column}>
                    <h4 className={styles.label}>{label}</h4>
                    {text ? (
                        <p className={styles.text}>
                            {text[0]}{' '}
                            {icons.map(icon => (
                                <span key={icon.name}>
                                    <i className={cx(classes(id, icon.name), styles.glyphBox)} />{' '}
                                </span>
                            ))}
                            {text[1]}
                        </p>
                    ) : codepoints ? (
                        <div className={styles.grid}>
                            {codepoints.map(codepoint => (
                                <figure key={codepoint} className={styles.card}>
                                    <span className={styles.glyph} style={{ fontFamily: `sa-${id}` }}>
                                        {String.fromCodePoint(codepoint)}
                                    </span>
                                    <figcaption>
                                        <code>{hex(codepoint)}</code>
                                    </figcaption>
                                </figure>
                            ))}
                        </div>
                    ) : (
                        <div className={styles.grid}>
                            {icons.map(icon => (
                                <figure key={icon.name} className={styles.card}>
                                    <div className={styles.pair}>
                                        <div className={styles.source} title={`${icon.name}.svg in the browser`} dangerouslySetInnerHTML={{ __html: icon.svg }} />
                                        <span className={styles.arrow} aria-hidden="true">
                                            →
                                        </span>
                                        <i className={cx(classes(id, icon.name), styles.glyph)} title={`${icon.name} in the font`} />
                                    </div>
                                    <figcaption>
                                        <code>icon-{displayName(icon.name)}</code>
                                    </figcaption>
                                </figure>
                            ))}
                        </div>
                    )}
                </section>
            );
        })}
    </div>
);
