import { no, type PageNav } from '../../site/nav.ts';

/** The previous and next pages in the current group. */
export function PrevNext({ nav }: { nav: PageNav }) {
    const { prev, next, numbered, index } = nav;
    return (
        <nav className="doc-next" aria-label="Previous and next pages">
            {prev ? (
                <a href={prev.link}>
                    <span className="mono">← {numbered ? `No. ${no(index - 1)}` : 'Previous'}</span>
                    <strong>{prev.text}</strong>
                </a>
            ) : (
                <span />
            )}
            {next && (
                <a className="right" href={next.link}>
                    <span className="mono">{numbered ? `No. ${no(index + 1)}` : 'Next'} →</span>
                    <strong>{next.text}</strong>
                </a>
            )}
        </nav>
    );
}
