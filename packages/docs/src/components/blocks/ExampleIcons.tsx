import type { FC } from 'react';
import { iconsFor } from './exampleSvgs.ts';
import styles from './ExampleIcons.module.css';

/** The input SVGs of one example, as a thumbnail grid labelled with their path inside `icons/`. */
export const ExampleIcons: FC<{ example: string }> = ({ example }) => (
    <div className={styles.icons}>
        {iconsFor(example).map(icon => (
            <figure key={icon.file} className={styles.icon}>
                <div className={styles.svg} dangerouslySetInnerHTML={{ __html: icon.svg }} />
                <figcaption>{icon.file}</figcaption>
            </figure>
        ))}
    </div>
);
