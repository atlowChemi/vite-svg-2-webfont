import { useId } from 'react';
import { letters, moltenSurface, tPath, tTransform, viewBox } from '../site/wordmark.ts';

/**
 * The fonte wordmark, inline rather than an `<img>` so the letters and the mold follow the site theme instead of the OS
 * color scheme. `surface` replaces the still molten surface (in the T's 100×86 box), e.g. with a frame of the landing
 * page's pour.
 */
export function Wordmark({ surface = moltenSurface }: { surface?: string }) {
    const clip = useId();
    return (
        <svg className="fonte-wordmark" viewBox={viewBox} role="img" aria-label="fonte">
            <path className="ink" d={letters} />
            <g transform={tTransform}>
                <clipPath id={clip}>
                    <path d={tPath} />
                </clipPath>
                <g clipPath={`url(#${clip})`}>
                    <path className="molten" d={`${surface} L100 100 L0 100 Z`} />
                    <path className="surface" d={surface} />
                </g>
                <path className="mold" d={tPath} />
            </g>
        </svg>
    );
}
