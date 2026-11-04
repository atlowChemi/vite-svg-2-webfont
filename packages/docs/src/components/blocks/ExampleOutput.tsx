import { Fragment } from 'react';
import { iconsFor } from './exampleIcons.ts';
import { Html } from './Html.tsx';

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
 * utilities). The generated CSS is scoped to this container, so markup uses real class names.
 */
export function ExampleOutput({ example, baseClass = 'icon', classPrefix = 'icon-', rows = [{ label: '' }], slots }: Props) {
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
        <div className={`example-output vp-example-${example}`}>
            {rows.map(row => (
                <Fragment key={row.label}>
                    {row.label && <p className="example-output-label">{row.label}</p>}
                    <div className="example-output-grid">
                        {names.map(name => (
                            <figure key={name}>
                                <i className={[baseClass, `${classPrefix}${name}`, row.class].filter(Boolean).join(' ')} aria-hidden="true" />
                                <figcaption>
                                    {classPrefix}
                                    {name}
                                </figcaption>
                            </figure>
                        ))}
                    </div>
                </Fragment>
            ))}
            {slots.default && <Html className="example-output-grid example-output-extra" html={slots.default} />}
        </div>
    );
}
