import { useState, type FC } from 'react';
import { cx } from '../cx.ts';
import styles from './PlaygroundToast.module.css';

interface Props {
    message?: string;
    onDismiss: () => void;
}

/** A notice in the corner, with a dismiss button; it fades in, and keeps its last message while it fades out. */
export const PlaygroundToast: FC<Props> = ({ message, onDismiss }) => {
    const [shown, setShown] = useState(message);
    if (message && message !== shown) setShown(message);
    if (!shown) return null;
    const leaving = !message;
    return (
        <div className={cx(styles.toast, leaving && styles.leaving)} role="alert" onAnimationEnd={() => leaving && setShown(undefined)}>
            <span>{shown}</span>
            <button type="button" aria-label="Dismiss" onClick={onDismiss}>
                ×
            </button>
        </div>
    );
};
