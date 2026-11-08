import { useEffect, useRef, useState, type FC } from 'react';
import styles from './PlaygroundToast.module.css';

interface Props {
    message?: string;
    onDismiss: () => void;
}

/**
 * A notice in the corner, with a dismiss button: a manual popover, so it sits in the top layer and CSS fades it in and
 * out. It keeps its last message while it fades out.
 */
export const PlaygroundToast: FC<Props> = ({ message, onDismiss }) => {
    const [shown, setShown] = useState(message);
    if (message && message !== shown) setShown(message);
    const toast = useRef<HTMLDivElement>(null);
    useEffect(() => {
        toast.current!.togglePopover(!!message);
    }, [message]);
    return (
        <div ref={toast} className={styles.toast} popover="manual" role="alert">
            <span>{shown}</span>
            <button type="button" aria-label="Dismiss" onClick={onDismiss}>
                ×
            </button>
        </div>
    );
};
