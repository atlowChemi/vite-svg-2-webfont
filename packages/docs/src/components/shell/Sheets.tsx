import { Fragment } from 'react';
import type { Heading } from '../../build/markdown.ts';
import { no, toolLink, type PageNav } from '../../site/nav.ts';
import { normalize, site, tools } from '../../site/site.ts';
import { filterable } from './Rail.tsx';

// Phone sheets. Each is a native modal dialog: the browser moves focus in, keeps it there, and closes it on Escape.

/** The guide and every tool; the current one lists its pages underneath, in place of the page strip. */
export function ToolsSheet({ nav }: { nav: PageNav }) {
    const entries = [
        { key: 'guide', plate: 'Start here', name: 'Guide', pkg: 'Concepts and features', link: '/guide/' },
        ...tools.map(t => ({ key: t.key, plate: t.plate, name: t.name, pkg: t.package, link: toolLink(t.key, nav.route) })),
    ];
    return (
        <dialog id="doc-tools" className="doc-sheet" aria-labelledby="doc-tools-title" tabIndex={-1}>
            <div id="doc-tools-title" className="doc-sheet-title mono">
                Tools
            </div>
            {entries.map(entry => (
                <Fragment key={entry.key}>
                    <a className="doc-sheet-tool" href={entry.link} aria-current={entry.key === nav.section ? 'true' : undefined}>
                        <span className="plate">{entry.plate}</span>
                        <strong>{entry.name}</strong>
                        <span className="pkg mono">{entry.pkg}</span>
                    </a>
                    {entry.key === nav.section && nav.groups.length > 0 && (
                        <div className="doc-sheet-pages">
                            {nav.groups.map(g => (
                                <Fragment key={g.text}>
                                    {nav.groups.length > 1 && <div className="doc-sheet-shelf mono">{g.text}</div>}
                                    {g.items.map(item => (
                                        <a key={item.link} className="doc-sheet-page" href={item.link} aria-current={normalize(item.link) === nav.route ? 'page' : undefined}>
                                            {item.text}
                                        </a>
                                    ))}
                                </Fragment>
                            ))}
                        </div>
                    )}
                </Fragment>
            ))}
            <a className="doc-sheet-tool play" href="/playground">
                <span className="plate">Try it</span>
                <strong>Playground</strong>
                <span className="pkg mono">Generate a font in your browser</span>
            </a>
            <div className="doc-sheet-links">
                <a href={site.github} target="_blank" rel="noreferrer">
                    <span className="plate">Source</span>GitHub ↗
                </a>
                {nav.tool && (
                    <a href={nav.tool.registry} target="_blank" rel="noreferrer">
                        <span className="plate">Package</span>
                        {nav.tool.registry.includes('crates.io') ? 'crates.io' : 'npm'} ↗
                    </a>
                )}
                <button type="button" data-theme-toggle="">
                    <span className="plate">Theme</span>
                    <span className="label">Switch</span>
                </button>
            </div>
        </dialog>
    );
}

/** Phones: the rail's index becomes a pill that names the current section; it opens the sections sheet. */
export function SectionPill({ headings }: { headings: Heading[] }) {
    return (
        <button className="doc-pill" type="button" aria-haspopup="dialog" aria-expanded="false" data-sheet="doc-sections">
            <span className="num">01</span>
            <span className="text">{headings[0]!.title}</span>
            <span aria-hidden="true">▴</span>
        </button>
    );
}

export function SectionsSheet({ name, headings }: { name: string; headings: Heading[] }) {
    return (
        <dialog id="doc-sections" className="doc-sheet" aria-labelledby="doc-sections-title" tabIndex={-1}>
            <div id="doc-sections-title" className="doc-sheet-title mono">
                On this page · {name}
            </div>
            {filterable(headings) && <input className="doc-filter" type="search" placeholder="Filter this page" aria-label="Filter this page's sections" />}
            {headings.map((s, i) => (
                <Fragment key={s.slug}>
                    <a className="doc-sheet-section" href={`#${s.slug}`}>
                        <span className="n">{no(i)}</span>
                        {s.title}
                    </a>
                    {s.children.map(c => (
                        <a key={c.slug} className="doc-sheet-section child" href={`#${c.slug}`}>
                            {c.title}
                        </a>
                    ))}
                </Fragment>
            ))}
        </dialog>
    );
}
