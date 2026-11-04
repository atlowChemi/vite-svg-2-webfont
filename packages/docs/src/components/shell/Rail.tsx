import type { Heading } from '../../build/markdown.ts';
import { no, type PageNav } from '../../site/nav.ts';

/** Ten entries or more get a filter. */
export const filterable = (headings: Heading[]) => headings.reduce((n, s) => n + s.children.length, 0) >= 10;

/**
 * The rail's index: the page's h2 sections, each with its h3 entries (options, functions). Every entry is in the markup;
 * the page script shows only the current section's entries and follows the reader.
 */
function SectionIndex({ headings }: { headings: Heading[] }) {
    return (
        <nav className="index" aria-label="On this page" data-index="">
            {filterable(headings) && <input className="doc-filter" type="search" placeholder="Filter this page" aria-label="Filter this page's sections" />}
            <ol>
                {headings.map((s, i) => (
                    <li key={s.slug}>
                        <a href={`#${s.slug}`}>
                            <span className="n">{no(i)}</span>
                            {s.title}
                        </a>
                        {s.children.length > 0 && (
                            <ol hidden>
                                {s.children.map(c => (
                                    <li key={c.slug}>
                                        <a href={`#${c.slug}`}>{c.title}</a>
                                    </li>
                                ))}
                            </ol>
                        )}
                    </li>
                ))}
            </ol>
            <p className="dim" hidden>
                Nothing matches.
            </p>
        </nav>
    );
}

/** The page's plate: its number or group, its name, its package; then the section index. */
export function Rail({ nav, name, headings }: { nav: PageNav; name: string; headings: Heading[] }) {
    const { tool, numbered, group, index } = nav;
    return (
        <aside className="doc-rail">
            {numbered ? (
                <>
                    <div className="mono dim">No.</div>
                    <div className="num">{no(index)}</div>
                </>
            ) : (
                <div className="mono dim">{tool ? tool.plate : group?.text}</div>
            )}
            <div className="name">{name}</div>
            {tool ? (
                <a className="mono dim tool" href={tool.registry} target="_blank" rel="noreferrer">
                    {tool.name} · {tool.package} ↗
                </a>
            ) : (
                numbered && <div className="mono dim tool">{group?.text}</div>
            )}
            {headings.length > 0 && <SectionIndex headings={headings} />}
        </aside>
    );
}
