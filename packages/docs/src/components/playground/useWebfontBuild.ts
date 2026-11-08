import { useEffect, useRef, useState } from 'react';
import type { PlaygroundIcon, PlaygroundSettings } from './options.ts';
import { createWebfontBuilder, type WebfontBuilder, type WebfontBuildState } from './webfontBuilder.ts';

/** Rebuild the font whenever the icons or settings change (see `createWebfontBuilder`); released on unmount. */
export function useWebfontBuild(icons: PlaygroundIcon[], settings: PlaygroundSettings): WebfontBuildState {
    const [state, setState] = useState<WebfontBuildState>({ building: false });
    const builder = useRef<WebfontBuilder>(undefined);
    useEffect(() => {
        const created = createWebfontBuilder(setState);
        builder.current = created;
        return () => {
            created.dispose();
            builder.current = undefined;
        };
    }, []);
    // Declared after the effect that creates the builder, so on mount it runs second and builds right away.
    useEffect(() => builder.current?.update(icons, settings), [icons, settings]);
    return state;
}
