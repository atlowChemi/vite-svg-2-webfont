import type { ReactNode } from 'react';
import { degrees } from './pour.ts';

/**
 * One numbered section of the specimen sheet: a sticky rail with the section's number, name and the page's temperature
 * (the landing script updates it as the page cools), then the content.
 */
export function Section({ num, name, id, className, children }: { num: string; name: string; id?: string; className?: string; children: ReactNode }) {
    return (
        <section id={id} className={className ? `landing-sec ${className}` : 'landing-sec'}>
            <div className="landing-rail mono">
                <span className="no">No.</span>
                <span className="num">{num}</span>
                <span className="name">{name}</span>
                <span className="deg">{degrees(1)}</span>
            </div>
            {children}
        </section>
    );
}

/**
 * A section's title and lede, with an illustration beside them when there is one. Illustrations load lazily: they sit
 * below the hero, and React would otherwise emit a preload link for each.
 */
export function SectionHeading({ title, lede, illustration }: { title: string; lede: string; illustration?: string }) {
    const text = (
        <>
            <h2 className="landing-title">{title}</h2>
            <p className="landing-lede">{lede}</p>
        </>
    );
    if (!illustration) return text;
    return (
        <div className="landing-heading">
            <img className="landing-illustration" src={illustration} alt="" loading="lazy" />
            <div>{text}</div>
        </div>
    );
}
