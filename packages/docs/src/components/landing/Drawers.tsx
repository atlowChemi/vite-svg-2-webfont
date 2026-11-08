import type { FC } from 'react';
import type { SiteIcon as IconName } from '../../site/siteIcons.ts';
import { SiteIcon } from '../SiteIcon.tsx';
import styles from './Drawers.module.css';

export interface Drawer {
    /** The snippet's slot name in index.md. */
    key: string;
    mark: IconName;
    plate: string;
    title: string;
    use: string;
    link: string;
}

export const toolDrawers: Drawer[] = [
    { key: 'cli', mark: 'rust', plate: 'Rust', title: 'CLI', use: 'CI jobs or one-off local runs, from any language', link: '/cli/' },
    { key: 'node', mark: 'node', plate: 'Node', title: 'Build scripts', use: 'Your own tooling, at native speed', link: '/node/' },
    { key: 'wasm', mark: 'wasm', plate: 'WASM', title: 'Browser', use: 'Casts from in-memory files, no file system needed', link: '/browser/' },
    { key: 'vite', mark: 'vite', plate: 'Vite', title: 'Plugin', use: 'Runs inside the dev server, icon edits hot-reload on save', link: '/vite/' },
];

interface Props {
    drawers: Drawer[];
    snippets: Record<string, string>;
}

/** The drawers `Drawers.module.css` can switch between: it shows a drawer's snippet and link by position, a rule each. */
export const maxDrawers = 4;

/**
 * The type case's drawers, one per way to run the engine, each with its install snippet and guide link. Each drawer is
 * a radio, and CSS shows the checked one's snippet and link: every panel is in the markup, and the first starts open.
 */
export const ToolDrawers: FC<Props> = ({ drawers, snippets }) => {
    if (drawers.length > maxDrawers) throw new Error(`The landing page has ${drawers.length} drawers; Drawers.module.css switches between at most ${maxDrawers}.`);
    return (
        <div className={styles.chooser}>
            <div className={styles.drawers} role="radiogroup" aria-label="How you run the engine">
                {drawers.map((drawer, i) => (
                    <label key={drawer.key} className={styles.drawer}>
                        <input className={styles.radio} type="radio" name="tool-drawer" value={drawer.key} defaultChecked={i === 0} />
                        <span className={styles.top}>
                            <SiteIcon name={drawer.mark} className={styles.mark} />
                            <span className={styles.plate}>{drawer.plate}</span>
                        </span>
                        <strong>{drawer.title}</strong>
                        <span className={styles.use}>{drawer.use}</span>
                    </label>
                ))}
            </div>
            {/* Snippets come from index.md as slots, so they are highlighted like any other docs code block. */}
            <div className={styles.snippet} aria-live="polite">
                {drawers.map(drawer => (
                    <div key={drawer.key} className={styles.panel} dangerouslySetInnerHTML={{ __html: snippets[drawer.key] ?? '' }} />
                ))}
            </div>
            <div>
                {drawers.map(drawer => (
                    <a key={drawer.key} className={styles.more} href={drawer.link}>
                        Read the {drawer.title} guide →
                    </a>
                ))}
            </div>
        </div>
    );
};
