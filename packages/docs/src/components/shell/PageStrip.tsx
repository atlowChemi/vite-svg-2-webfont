import { no, type PageNav } from '../../site/nav.ts';
import { normalize } from '../../site/site.ts';

/** The current group's pages as tabs; the guide adds a switch between its two shelves. Hidden on phones. */
export function PageStrip({ nav }: { nav: PageNav }) {
    const { group, groups, isGuide, numbered, route } = nav;
    return (
        <nav className="doc-strip" aria-label={`${group?.text ?? ''} pages`}>
            {isGuide && (
                <span className="doc-shelves" role="group" aria-label="Guide shelves">
                    {groups.map(g => (
                        <a key={g.text} href={g.items[0]!.link} aria-current={g === group ? 'true' : undefined}>
                            {g.text}
                        </a>
                    ))}
                </span>
            )}
            {group?.items.map((item, i) => (
                <a key={item.link} href={item.link} aria-current={normalize(item.link) === route ? 'page' : undefined}>
                    {numbered && <span className="n">{no(i)}</span>}
                    {item.text}
                </a>
            ))}
            {!isGuide && (
                <a className="guide" href="/guide/">
                    Guide &amp; features →
                </a>
            )}
        </nav>
    );
}
