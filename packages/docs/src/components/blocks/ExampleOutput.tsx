import { Fragment, type FC } from 'react';
import { exampleScope } from '../../site/exampleScope.ts';
import { cx } from '../cx.ts';
import { iconsFor } from './exampleSvgs.ts';
import styles from './ExampleOutput.module.css';

interface Props {
    example: string;
    baseClass?: string;
    classPrefix?: string;
    /** Repeats the icons once per extra class, such as a design modifier. */
    rows?: { label: string; class?: string }[];
    slots: Record<string, string>;
}

/**
 * Live output of one example: every input icon with its class, then any slot content (for example a template's
 * utilities). The generated CSS is scoped to this container, so the icons use the example's real class names.
 */
export const ExampleOutput: FC<Props> = ({ example, baseClass = 'icon', classPrefix = 'icon-', rows = [{ label: '' }], slots }) => {
    const names = [
        ...new Set(
            iconsFor(example).map(icon =>
                icon.file
                    .split('/')
                    .at(-1)!
                    .replace(/\.svg$/, ''),
            ),
        ),
    ];
    return (
        <div className={cx(styles.output, exampleScope(example))}>
            {rows.map(row => (
                <Fragment key={row.label}>
                    {row.label && <p className={styles.label}>{row.label}</p>}
                    <div className={styles.grid}>
                        {names.map(name => (
                            <figure key={name}>
                                <i className={cx(baseClass, `${classPrefix}${name}`, row.class)} aria-hidden="true" />
                                <figcaption>
                                    {classPrefix}
                                    {name}
                                </figcaption>
                            </figure>
                        ))}
                    </div>
                </Fragment>
            ))}
            {slots.default && <div className={cx(styles.grid, styles.extra)} dangerouslySetInnerHTML={{ __html: slots.default }} />}
        </div>
    );
};
