import type { FC } from 'react';
import styles from './Features.module.css';

export interface Feature {
    icon: string;
    title: string;
    details: string;
}

export const foundryFeatures: Feature[] = [
    {
        icon: '/native-workflow.svg',
        title: 'Native speed',
        details: 'A ground-up Rust rewrite of the webfont pipeline, with parallel font conversion and native path normalization.',
    },
    {
        icon: '/configurable.svg',
        title: 'Every detail configurable',
        details: 'File globs, destinations, selectors, class prefixes, codepoints, font formats, and your own templates.',
    },
    {
        icon: '/in-browser.svg',
        title: 'Right in the browser',
        details: 'The same engine compiled to WebAssembly casts fonts in a web worker, with no server and no build step.',
    },
];

interface Props {
    features: Feature[];
}

/** The foundry's selling points, one illustrated column each. */
export const Features: FC<Props> = ({ features }) => (
    <div className={styles.features}>
        {features.map(feature => (
            <div key={feature.title} className={styles.feature}>
                <img className={styles.illustration} src={feature.icon} alt="" loading="lazy" />
                <h3>{feature.title}</h3>
                <p>{feature.details}</p>
            </div>
        ))}
    </div>
);
