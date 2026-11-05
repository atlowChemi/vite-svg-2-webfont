import type { FC } from 'react';
import controls from './controls.module.css';
import styles from './PlaygroundPage.module.css';

/** The element `src/client/playground.tsx` renders the Playground into. */
// Not `playground`: the page's "Playground" heading already has that id.
export const PLAYGROUND_ROOT_ID = 'playground-app';

/**
 * The Playground's page: the heading and introduction from playground.md (`intro`), then the app's place. The app
 * renders in the browser only, replacing the fallback, which stays for readers without JavaScript.
 */
export const PlaygroundPage: FC<{ intro: string }> = ({ intro }) => (
    <div className={styles.page}>
        <div className={styles.intro} dangerouslySetInnerHTML={{ __html: intro }} />
        <div id={PLAYGROUND_ROOT_ID}>
            <p className={controls.hint}>The Playground runs in your browser and needs JavaScript. It starts once the page has loaded.</p>
        </div>
    </div>
);
