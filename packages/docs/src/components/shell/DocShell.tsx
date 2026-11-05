import type { FC, ReactNode } from 'react';
import type { Heading } from '../../build/markdown.ts';
import type { PageNav } from '../../site/nav.ts';
import { site } from '../../site/site.ts';
import { cx } from '../cx.ts';
import styles from './DocShell.module.css';
import { PageStrip } from './PageStrip.tsx';
import { PrevNext } from './PrevNext.tsx';
import prose from './Prose.module.css';
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

const SkipLink: FC = () => (
    <a className={styles.skip} href="#main-content">
        Skip to content
    </a>
);

/** A doc page: top bar, page strip, rail and the page's HTML; on phones, a section pill and sheets. */
export const DocShell: FC<{ page: DocPage }> = ({ page }) => {
    const { nav, headings } = page;
    const name = nav.group?.items[nav.index]?.text ?? page.title;
    return (
        <div className={styles.frame}>
            <SkipLink />
            <TopBar nav={nav} />
            <PageStrip nav={nav} />
            <div className={styles.layout}>
                <Rail nav={nav} name={name} headings={headings} />
                <main id="main-content" className={styles.main} tabIndex={-1}>
                    <div className={prose.prose} dangerouslySetInnerHTML={{ __html: page.html }} data-content="" />
                    <a className={styles.edit} href={site.editPattern.replace(':path', page.file)} target="_blank" rel="noreferrer">
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
};

interface PageShellProps {
    nav: PageNav;
    /** Ends in the site's footer; the landing page has a footer of its own. */
    footer?: boolean;
    children: ReactNode;
}

/** A full-width page under the same top bar: the landing page, the Playground and the 404 page. */
export const PageShell: FC<PageShellProps> = ({ nav, footer = true, children }) => (
    <div className={cx(styles.frame, footer && styles.footed)}>
        <SkipLink />
        <TopBar nav={nav} />
        <main id="main-content" className={styles.wideMain} tabIndex={-1}>
            {children}
        </main>
        {footer && <SiteFooter />}
        <ToolsSheet nav={nav} />
    </div>
);
