import { useState, type ChangeEvent, type DragEvent } from 'react';

/** Where SVGs come in: dropped, picked, or the sample set. */
export function PlaygroundDropZone({ onFiles, onSamples }: { onFiles: (files: Iterable<File>) => void; onSamples: () => void }) {
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
            className={dragging ? 'pg-drop pg-drop-active' : 'pg-drop'}
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
            <div className="pg-drop-actions">
                <label className="pg-button">
                    Choose files
                    <input className="pg-visually-hidden" type="file" accept=".svg,image/svg+xml" multiple onChange={onPick} />
                </label>
                <button type="button" className="pg-button pg-button-alt" onClick={onSamples}>
                    Load sample icons
                </button>
            </div>
            <p className="pg-hint">Everything runs in your browser. Your SVGs are never uploaded.</p>
        </div>
    );
}
