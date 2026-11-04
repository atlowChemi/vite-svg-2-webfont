import { CommitInput } from './CommitInput.tsx';
import { formatCodepoint, isPrivateUse } from './codepoints.ts';
import type { PlaygroundIcon } from './options.ts';

export type PlaygroundIconCardProps = {
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
};

/**
 * One icon: its preview, editable name and codepoint, and class name. `previewFamily` renders the glyph from the
 * built font when it contains this icon (`built`). Without a previewable font, `showSource` shows the source SVG
 * instead; otherwise an empty placeholder keeps the card's size.
 */
export function PlaygroundIconCard({
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
}: PlaygroundIconCardProps) {
    const box = { width: `${size}px`, height: `${size}px` };
    return (
        <figure className={nameError || codepointError ? 'pg-card pg-card-invalid' : 'pg-card'}>
            <button className="pg-card-remove" type="button" aria-label={`Remove ${icon.name}`} title="Remove" onClick={onRemove}>
                ×
            </button>
            {previewFamily && built ? (
                <span className="pg-card-glyph" style={{ fontFamily: previewFamily, fontSize: `${size}px` }}>
                    {String.fromCodePoint(icon.codepoint)}
                </span>
            ) : showSource ? (
                // The viewer's own SVG, shown only in their browser.
                <span className="pg-card-glyph pg-card-source" style={box} dangerouslySetInnerHTML={{ __html: icon.svg }} />
            ) : (
                <span className="pg-card-glyph" style={box} aria-hidden="true" />
            )}
            <label className="pg-card-field">
                <span className="pg-visually-hidden">Name</span>
                <CommitInput value={icon.name} spellCheck={false} aria-invalid={Boolean(nameError)} onCommit={onRename} />
            </label>
            <code className="pg-card-class">
                {classPrefix}
                {icon.name}
            </code>
            <label className="pg-card-field">
                <span className="pg-visually-hidden">Codepoint</span>
                <CommitInput
                    value={formatCodepoint(icon.codepoint)}
                    className="pg-card-codepoint"
                    spellCheck={false}
                    aria-invalid={Boolean(codepointError)}
                    onCommit={onCodepoint}
                />
            </label>
            {nameError || codepointError ? (
                <small className="pg-card-error">{nameError ?? codepointError}</small>
            ) : (
                !isPrivateUse(icon.codepoint) && <small className="pg-card-warning">Outside the Private Use Area</small>
            )}
        </figure>
    );
}
