import type { ReactNode } from 'react';
import type { Heading } from '../../build/markdown.ts';
import type { PageNav } from '../../site/nav.ts';
import { site } from '../../site/site.ts';
import { PageStrip } from './PageStrip.tsx';
import { PrevNext } from './PrevNext.tsx';
import { Rail } from './Rail.tsx';
import { SectionPill, SectionsSheet, ToolsSheet } from './Sheets.tsx';
import { SiteFooter } from './SiteFooter.tsx';
import { TopBar } from './TopBar.tsx';

export interface DocPage {
    nav: PageNav;
    title: string;
    /** The page's source under packages/docs, for the edit link. */
    file: string;
    headings: Heading[];
    html: string;
}

/** A doc page: top bar, page strip, rail and the page's HTML; on phones, a section pill and sheets. */
export function DocShell({ page }: { page: DocPage }) {
    const { nav, headings } = page;
    const name = nav.group?.items[nav.index]?.text ?? page.title;
    return (
        <div className="doc">
            <a className="doc-skip" href="#doc-content">
                Skip to content
            </a>
            <TopBar nav={nav} />
            <PageStrip nav={nav} />
            <div className="doc-page">
                <Rail nav={nav} name={name} headings={headings} />
                <main id="doc-content" className="doc-main" tabIndex={-1}>
                    <div className="vp-doc" dangerouslySetInnerHTML={{ __html: page.html }} />
                    <a className="doc-edit" href={site.editPattern.replace(':path', page.file)} target="_blank" rel="noreferrer">
                        Edit this page on GitHub ↗
                    </a>
                    <PrevNext nav={nav} />
                </main>
            </div>
            {headings.length > 0 && <SectionPill headings={headings} />}
            <ToolsSheet nav={nav} />
            {headings.length > 0 && <SectionsSheet name={name} headings={headings} />}
        </div>
    );
}

/** A full-width page under the same top bar: the landing page, the Playground and the 404 page. */
export function PageShell({ nav, footer = true, children }: { nav: PageNav; footer?: boolean; children: ReactNode }) {
    return (
        <div className={footer ? 'doc doc-wide doc-footed' : 'doc doc-wide'}>
            <a className="doc-skip" href="#doc-content">
                Skip to content
            </a>
            <TopBar nav={nav} />
            <main id="doc-content" className="doc-wide-main" tabIndex={-1}>
                {children}
            </main>
            {footer && <SiteFooter />}
            <ToolsSheet nav={nav} />
        </div>
    );
}
