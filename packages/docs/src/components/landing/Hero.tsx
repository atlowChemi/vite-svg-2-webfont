import { codepointLabel, trayIcons, type SiteIcon as IconName } from '../../site/siteIcons.ts';
import { SiteIcon } from '../SiteIcon.tsx';
import { Wordmark } from '../Wordmark.tsx';
import { emptySurface } from './pour.ts';
import { Section } from './Section.tsx';

/**
 * The first section: a tray of icons above the empty mold, which the landing script pours into the T as the reader
 * scrolls through the pinned stage, revealing the cast font at the end.
 */
export function Hero() {
    return (
        <Section num="01" name="Casting" className="landing-hero">
            <div className="landing-stage">
                <div className="landing-tray" aria-hidden="true">
                    {trayIcons.map(name => (
                        <span key={name} className="landing-tray-item">
                            <SiteIcon name={name} />
                        </span>
                    ))}
                </div>
                <div className="landing-placeholder">
                    <Wordmark surface={emptySurface} />
                </div>
                <h1 className="landing-tag">Melted SVGs, cast into fonts</h1>
                <p className="landing-sub">A Rust engine for icon fonts, for the CLI, Node, the browser, and Vite.</p>
                <div className="landing-hint mono" aria-hidden="true">
                    <span>scroll to cast</span>
                    <SiteIcon name="arrow-down" />
                </div>
                <CastResult fileName="my-icons.woff2" icons={trayIcons} />
            </div>
        </Section>
    );
}

/** The font the pour casts: its file, a cell per glyph with its codepoint, and where to go next. */
export function CastResult({ fileName, icons }: { fileName: string; icons: readonly IconName[] }) {
    return (
        <div className="landing-result">
            <div className="landing-result-head mono">
                <span>{fileName}</span>
                <span>{icons.length} glyphs</span>
            </div>
            <div className="landing-cells">
                {icons.map(name => (
                    <div key={name} className="landing-cell">
                        <SiteIcon name={name} />
                        <code className="mono">U+{codepointLabel(name)}</code>
                    </div>
                ))}
            </div>
            <div className="landing-actions">
                <a className="landing-button brand" href="/guide/">
                    Get started
                </a>
                <a className="landing-button" href="/playground">
                    Try the playground
                </a>
            </div>
        </div>
    );
}
