import type { CSSProperties } from 'react';
import { codepointLabel, trayIcons } from '../site/siteIcons.ts';
import { SiteIcon } from './SiteIcon.tsx';

// A type case of glyphs cast in the site's icon font, with one empty compartment: the sort for this page. A highlight
// scans the case once and stops on the gap; with reduced motion, the gap is marked straight away. NotFound.css runs the
// scan from each cell's `--i`.
const missing = 10;
const cells = Array.from({ length: 18 }, (_, i) => trayIcons[(i * 5) % trayIcons.length]!);

/** The 404 page: what went missing, the way back, and the case with its empty compartment. */
export function NotFound() {
    return (
        <div className="not-found">
            <div className="not-found-copy">
                <h1>
                    <span className="code">404</span>
                    <span className="line">This sort isn’t in the case.</span>
                </h1>
                <p className="lede">A sort is one piece of cast type. The page you asked for isn’t in this case: it may have moved when the docs were reorganized by tool.</p>
                <p className="mono where">U+E404 · not found</p>
                <div className="actions">
                    <a className="not-found-button brand" href="/guide/">
                        Go to the guide
                    </a>
                    <a className="not-found-button" href="/">
                        Home
                    </a>
                </div>
            </div>
            <div className="not-found-case" aria-hidden="true">
                {cells.map((cell, i) =>
                    i === missing ? (
                        <div key={i} className="cell hole" style={{ '--i': i } as CSSProperties} />
                    ) : (
                        // Only the cells before the gap are scanned.
                        <div key={i} className={i < missing ? 'cell scan' : 'cell'} style={i < missing ? ({ '--i': i } as CSSProperties) : undefined}>
                            <SiteIcon name={cell} />
                            <span className="mono">{codepointLabel(cell)}</span>
                        </div>
                    ),
                )}
            </div>
        </div>
    );
}
