import { useEffect, useMemo, useState } from 'react';
import { highlight } from './highlight.ts';
import { useClipboard } from './useClipboard.ts';
import type { WebfontBuild } from './webfontBuilder.ts';

type ViewerTab = 'css' | 'html' | 'svg' | 'ts';

/** Highlighting very large output (an SVG font with many icons) would stall the page; it then stays plain text. */
const HIGHLIGHT_LIMIT = 200_000;

/** The generated CSS, HTML, SVG font and TypeScript, in tabs, highlighted like the site's code blocks. */
export function PlaygroundViewer({ build }: { build: WebfontBuild }) {
    const [viewer, setViewer] = useState<ViewerTab>('css');
    const { copied, copy } = useClipboard();

    /** The SVG font is the one text-based font format, so it can be shown alongside CSS, HTML and TypeScript. */
    const svgFont = useMemo(() => (build.outputs.svg ? new TextDecoder().decode(build.outputs.svg) : undefined), [build.outputs.svg]);
    // The SVG font tab goes away with the format; fall back to CSS, and stay there if it comes back.
    if (!svgFont && viewer === 'svg') setViewer('css');
    const tab = !svgFont && viewer === 'svg' ? 'css' : viewer;
    const viewerText = { css: build.css, html: build.html, ts: build.ts, svg: svgFont }[tab] ?? '';

    const [highlighted, setHighlighted] = useState<{ text: string; html: string }>();
    // Highlight the visible output once Shiki loads; plain text shows until then, and stale results are dropped.
    useEffect(() => {
        if (!viewerText || viewerText.length > HIGHLIGHT_LIMIT) return undefined;
        let current = true;
        highlight(viewerText, tab === 'svg' ? 'xml' : tab).then(
            html => current && setHighlighted({ text: viewerText, html }),
            // Highlighting is cosmetic; the plain text stays visible.
            () => {},
        );
        return () => {
            current = false;
        };
    }, [viewerText, tab]);

    const tabProps = (name: ViewerTab) => ({ type: 'button' as const, role: 'tab', 'data-title': `output.${name}`, 'aria-selected': tab === name, onClick: () => setViewer(name) });

    return (
        <div className="pg-viewer">
            <div className="pg-viewer-tabs">
                {/* A tab list holds only tabs; Copy sits beside it. */}
                <div className="pg-viewer-tablist" role="tablist" aria-label="Generated code">
                    <button {...tabProps('css')}>CSS</button>
                    <button {...tabProps('html')}>HTML</button>
                    {svgFont && <button {...tabProps('svg')}>SVG font</button>}
                    <button {...tabProps('ts')}>TypeScript</button>
                </div>
                <button type="button" className="pg-link" onClick={() => void copy('source', viewerText)}>
                    {copied === 'source' ? 'Copied!' : 'Copy'}
                </button>
            </div>
            {highlighted?.text === viewerText ? (
                <div className="pg-code pg-code-highlighted" dangerouslySetInnerHTML={{ __html: highlighted.html }} />
            ) : (
                <pre className="pg-code">
                    <code>{viewerText}</code>
                </pre>
            )}
        </div>
    );
}
