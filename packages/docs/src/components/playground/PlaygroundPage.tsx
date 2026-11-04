/** The element `src/client/playground.tsx` renders the Playground into. */
// Not `playground`: the page's "Playground" heading already has that id.
export const PLAYGROUND_ROOT_ID = 'playground-app';

/**
 * The Playground's place in the static page. The app renders in the browser only, replacing the fallback, which stays
 * for readers without JavaScript.
 */
export function PlaygroundPage() {
    return (
        <div id={PLAYGROUND_ROOT_ID}>
            <p className="pg-hint">The Playground runs in your browser and needs JavaScript. It starts once the page has loaded.</p>
        </div>
    );
}
