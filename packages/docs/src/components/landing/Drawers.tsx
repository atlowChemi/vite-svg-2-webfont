import type { SiteIcon as IconName } from '../../site/siteIcons.ts';
import { SiteIcon } from '../SiteIcon.tsx';

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

/**
 * The type case's drawers, one per way to run the engine, each with its install snippet and guide link. Every panel is
 * in the markup and the first is open; the landing script switches them.
 */
export function ToolDrawers({ drawers, snippets }: { drawers: Drawer[]; snippets: Record<string, string> }) {
    return (
        <>
            <div className="landing-drawers">
                {drawers.map((drawer, i) => (
                    <button key={drawer.key} className="landing-drawer" type="button" aria-pressed={i === 0} data-drawer={drawer.key}>
                        <span className="drawer-top">
                            <SiteIcon name={drawer.mark} className="mark" />
                            <span className="plate mono">{drawer.plate}</span>
                        </span>
                        <strong>{drawer.title}</strong>
                        <span className="use">{drawer.use}</span>
                    </button>
                ))}
            </div>
            {/* Snippets come from index.md as slots, so they are highlighted like any other docs code block. */}
            <div className="landing-snippet vp-doc" aria-live="polite">
                {drawers.map((drawer, i) => (
                    <div key={drawer.key} data-drawer-panel={drawer.key} hidden={i !== 0} dangerouslySetInnerHTML={{ __html: snippets[drawer.key] ?? '' }} />
                ))}
            </div>
            {drawers.map((drawer, i) => (
                <a key={drawer.key} className="landing-more" href={drawer.link} data-drawer-link={drawer.key} hidden={i !== 0}>
                    Read the {drawer.title} guide →
                </a>
            ))}
        </>
    );
}
