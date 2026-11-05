import type { FC } from 'react';
import { cx } from '../cx.ts';
import { CommitInput } from './CommitInput.tsx';
import styles from './PlaygroundIconCard.module.css';
import { formatCodepoint, isPrivateUse } from './codepoints.ts';
import controls from './controls.module.css';
import type { PlaygroundIcon } from './options.ts';

export interface PlaygroundIconCardProps {
    icon: PlaygroundIcon;
    previewFamily?: string;
    built?: boolean;
    showSource?: boolean;
    size: number;
    classPrefix: string;
    nameError?: string;
    codepointError?: string;
    onRename: (name: string) => void;
    onCodepoint: (text: string) => void;
    onRemove: () => void;
}

/**
 * One icon: its preview, editable name and codepoint, and class name. `previewFamily` renders the glyph from the
 * built font when it contains this icon (`built`). Without a previewable font, `showSource` shows the source SVG
 * instead; otherwise an empty placeholder keeps the card's size.
 */
export const PlaygroundIconCard: FC<PlaygroundIconCardProps> = ({
    icon,
    previewFamily,
    built,
    showSource,
    size,
    classPrefix,
    nameError,
    codepointError,
    onRename,
    onCodepoint,
    onRemove,
}) => {
    const box = { width: `${size}px`, height: `${size}px` };
    return (
        <figure className={cx(styles.card, (nameError || codepointError) && styles.invalid)}>
            <button className={styles.remove} type="button" aria-label={`Remove ${icon.name}`} title="Remove" onClick={onRemove}>
                ×
            </button>
            {previewFamily && built ? (
                <span className={styles.glyph} style={{ fontFamily: previewFamily, fontSize: `${size}px` }}>
                    {String.fromCodePoint(icon.codepoint)}
                </span>
            ) : showSource ? (
                // The viewer's own SVG, shown only in their browser.
                <span className={cx(styles.glyph, styles.source)} style={box} dangerouslySetInnerHTML={{ __html: icon.svg }} />
            ) : (
                <span className={styles.glyph} style={box} aria-hidden="true" />
            )}
            <label className={styles.field}>
                <span className={controls.visuallyHidden}>Name</span>
                <CommitInput value={icon.name} spellCheck={false} aria-invalid={Boolean(nameError)} onCommit={onRename} />
            </label>
            <code className={styles.iconClass}>
                {classPrefix}
                {icon.name}
            </code>
            <label className={styles.field}>
                <span className={controls.visuallyHidden}>Codepoint</span>
                <CommitInput
                    value={formatCodepoint(icon.codepoint)}
                    className={styles.codepoint}
                    spellCheck={false}
                    aria-invalid={Boolean(codepointError)}
                    onCommit={onCodepoint}
                />
            </label>
            {nameError || codepointError ? (
                <small className={styles.error}>{nameError ?? codepointError}</small>
            ) : (
                !isPrivateUse(icon.codepoint) && <small className={styles.warning}>Outside the Private Use Area</small>
            )}
        </figure>
    );
};
