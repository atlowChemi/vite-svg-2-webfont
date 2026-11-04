import { toolLink, type PageNav } from '../../site/nav.ts';
import { site, tools } from '../../site/site.ts';
import { SiteIcon } from '../SiteIcon.tsx';
import { Wordmark } from '../Wordmark.tsx';

/** The site's top bar: the wordmark, the tool drawers (a sheet on phones), the Playground, search, GitHub and the theme. */
export function TopBar({ nav }: { nav: PageNav }) {
    const pageCount = nav.groups.reduce((n, g) => n + g.items.length, 0);
    return (
        <header className="doc-top">
            <a className="doc-wordmark" href="/" aria-label="fonte home">
                <Wordmark />
            </a>
            <button className="doc-tool-button" type="button" aria-haspopup="dialog" aria-expanded="false" data-sheet="doc-tools">
                <span className="plate">{nav.tool?.plate ?? 'Start here'}</span>
                <strong>{nav.tool?.name ?? 'Guide'}</strong>
                {pageCount > 0 && <span className="cue mono">{pageCount} pages</span>}
                <SiteIcon name="chevron-down" className="caret" />
            </button>
            <nav className="doc-drawers" aria-label="Tools">
                <a className="doc-drawer" href="/guide/" aria-current={nav.isGuide ? 'true' : undefined}>
                    <span className="plate">Start here</span>
                    <strong>Guide</strong>
                </a>
                {tools.map(t => (
                    <a key={t.key} className="doc-drawer" href={toolLink(t.key, nav.route)} aria-current={t.key === nav.section ? 'true' : undefined}>
                        <span className="plate">{t.plate}</span>
                        <strong>{t.name}</strong>
                    </a>
                ))}
            </nav>
            <a className="doc-play" href="/playground" aria-label="Playground: generate a font in your browser">
                <SiteIcon name="play" className="mark" />
                <span className="label" aria-hidden="true">
                    <span className="plate">Try it</span>
                    <strong>Playground</strong>
                </span>
            </a>
            <div className="doc-search">
                <button type="button" className="doc-search-button" aria-label="Search" aria-keyshortcuts="/ Control+K Meta+K" data-search="">
                    <SiteIcon name="search" />
                    <span className="placeholder">Search</span>
                    <kbd className="mono">⌘K</kbd>
                </button>
            </div>
            <a className="doc-icon doc-github" href={site.github} target="_blank" rel="noreferrer" aria-label="fonte on GitHub">
                <SiteIcon name="github" />
            </a>
            <button className="doc-icon doc-theme" type="button" aria-label="Switch theme" title="Switch theme" data-theme-toggle="">
                <SiteIcon name="theme" />
            </button>
        </header>
    );
}
