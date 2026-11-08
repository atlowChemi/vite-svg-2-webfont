import { useState, type FC } from 'react';
import { cx } from '../cx.ts';
import styles from './PlaygroundOutputs.module.css';
import { PlaygroundViewer } from './PlaygroundViewer.tsx';
import { codepointsSnippet } from './codepoints.ts';
import controls from './controls.module.css';
import { downloadFile } from './download.ts';
import type { PlaygroundIcon, PlaygroundSettings } from './options.ts';
import { useClipboard } from './useClipboard.ts';
import type { WebfontBuild } from './webfontBuilder.ts';
import { outputFiles, zipOutputs } from './zip.ts';

interface Props {
    build: WebfontBuild;
    settings: PlaygroundSettings;
    icons: PlaygroundIcon[];
}

/** Downloads for the latest build, the codepoints snippet, and the generated code. */
export const PlaygroundOutputs: FC<Props> = ({ build, settings, icons }) => {
    const { copied, copy } = useClipboard();
    const [zipping, setZipping] = useState(false);
    const [zipError, setZipError] = useState<string>();

    async function downloadZip() {
        if (zipping) return;
        // Capture the current build: a rebuild can finish while the archive is being compressed.
        const { fontName } = settings;
        const { outputs } = build;
        setZipping(true);
        setZipError(undefined);
        try {
            downloadFile(`${fontName}.zip`, await zipOutputs(fontName, outputs));
        } catch (cause) {
            setZipError(`Could not create the ZIP: ${cause instanceof Error ? cause.message : String(cause)}`);
        } finally {
            setZipping(false);
        }
    }

    return (
        <section className={styles.outputs}>
            <h2>Outputs</h2>
            <div className={styles.files}>
                {outputFiles(build.outputs, settings).map(file => (
                    <button key={file.kind} type="button" className={styles.file} onClick={() => downloadFile(file.fileName, file.bytes)}>
                        <span>{file.fileName}</span>
                        <small>{file.size}</small>
                    </button>
                ))}
                <button type="button" className={controls.button} disabled={zipping} onClick={() => void downloadZip()}>
                    {zipping ? 'Zipping…' : 'Download all (.zip)'}
                </button>
                <button type="button" className={cx(controls.button, controls.buttonAlt)} onClick={() => void copy('codepoints', codepointsSnippet(icons))}>
                    {copied === 'codepoints' ? 'Copied!' : 'Copy as codepoints'}
                </button>
            </div>
            {zipError && (
                <p className={controls.error} role="alert">
                    {zipError}
                </p>
            )}
            <p className={controls.hint}>Codepoints are assigned in the order icons were added. Paste the copied option into your config to keep them stable.</p>
            <PlaygroundViewer build={build} />
        </section>
    );
};
