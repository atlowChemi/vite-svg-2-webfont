import { useState } from 'react';

/** A notice in the corner, with a dismiss button; it fades in, and keeps its last message while it fades out. */
export function PlaygroundToast({ message, onDismiss }: { message?: string; onDismiss: () => void }) {
    const [shown, setShown] = useState(message);
    if (message && message !== shown) setShown(message);
    if (!shown) return null;
    const leaving = !message;
    return (
        <div className={leaving ? 'pg-toast pg-toast-leaving' : 'pg-toast'} role="alert" onAnimationEnd={() => leaving && setShown(undefined)}>
            <span>{shown}</span>
            <button type="button" aria-label="Dismiss" onClick={onDismiss}>
                ×
            </button>
        </div>
    );
}
