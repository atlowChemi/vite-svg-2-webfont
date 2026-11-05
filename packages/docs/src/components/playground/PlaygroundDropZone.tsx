import { useState, type ChangeEvent, type DragEvent, type FC } from 'react';
import { cx } from '../cx.ts';
import styles from './PlaygroundDropZone.module.css';
import controls from './controls.module.css';

interface Props {
    onFiles: (files: Iterable<File>) => void;
    onSamples: () => void;
}

/** Where SVGs come in: dropped, picked, or the sample set. */
export const PlaygroundDropZone: FC<Props> = ({ onFiles, onSamples }) => {
    const [dragging, setDragging] = useState(false);

    function onDrop(event: DragEvent) {
        event.preventDefault();
        setDragging(false);
        onFiles(event.dataTransfer.files);
    }

    function onPick(event: ChangeEvent<HTMLInputElement>) {
        const input = event.target;
        // The FileList is live: clearing the input below empties it, so hand over a copy.
        if (input.files) onFiles([...input.files]);
        input.value = '';
    }

    return (
        <div
            className={cx(styles.drop, dragging && styles.active)}
            onDragOver={event => {
                event.preventDefault();
                setDragging(true);
            }}
            onDragLeave={() => setDragging(false)}
            onDrop={onDrop}
        >
            <p>
                <strong>Drop SVG files here</strong>
            </p>
            <div className={styles.actions}>
                <label className={controls.button}>
                    Choose files
                    <input className={controls.visuallyHidden} type="file" accept=".svg,image/svg+xml" multiple onChange={onPick} />
                </label>
                <button type="button" className={cx(controls.button, controls.buttonAlt)} onClick={onSamples}>
                    Load sample icons
                </button>
            </div>
            <p className={controls.hint}>Everything runs in your browser. Your SVGs are never uploaded.</p>
        </div>
    );
};
