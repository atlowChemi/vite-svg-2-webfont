import { Fragment, useState } from 'react';
import builtInCss from '../../../../webfont-generator/templates/css.hbs?raw';
import builtInHtml from '../../../../webfont-generator/templates/html.hbs?raw';
import { CommitInput } from './CommitInput.tsx';
import { formatCodepoint, parseCodepoint } from './codepoints.ts';
import { ALL_TYPES, selectedTypes, unavailableTypes, type ColorMode, type PlaygroundSettings } from './options.ts';
import { movedFormat, parseOptionalNumber, toggled, type NumberSetting } from './settingsEdits.ts';

type TextSetting = {
    [Key in keyof PlaygroundSettings]-?: PlaygroundSettings[Key] extends string ? Key : never;
}[keyof PlaygroundSettings];
type BooleanSetting = {
    [Key in keyof PlaygroundSettings]-?: PlaygroundSettings[Key] extends boolean ? Key : never;
}[keyof PlaygroundSettings];
type TemplateSetting = 'cssTemplate' | 'htmlTemplate';

const BUILT_IN_TEMPLATES: Record<TemplateSetting, string> = { cssTemplate: builtInCss, htmlTemplate: builtInHtml };

export type PlaygroundOptionsProps = {
    settings: PlaygroundSettings;
    iconNames: string[];
    onChange: (patch: Partial<PlaygroundSettings>) => void;
    onReassign: () => void;
    onReset: () => void;
};

/** Every generator option, grouped like the docs' option pages. */
export function PlaygroundOptions({ settings, iconNames, onChange, onReassign, onReset }: PlaygroundOptionsProps) {
    const unavailable = unavailableTypes(settings);
    const ordered = selectedTypes(settings);
    const [startError, setStartError] = useState<string>();

    const text = (key: TextSetting) => ({ value: settings[key], onChange: (event: { target: { value: string } }) => onChange({ [key]: event.target.value }) });
    const check = (key: BooleanSetting) => ({
        type: 'checkbox',
        checked: settings[key],
        onChange: (event: { target: { checked: boolean } }) => onChange({ [key]: event.target.checked }),
    });
    const number = (key: NumberSetting) => ({
        type: 'number',
        value: settings[key]?.toString() ?? '',
        onCommit: (value: string) => onChange({ [key]: parseOptionalNumber(value) }),
    });
    const colorMode = (mode: ColorMode) => ({ type: 'radio', value: mode, checked: settings.colorMode === mode, onChange: () => onChange({ colorMode: mode }) });

    function move(type: (typeof ordered)[number], step: -1 | 1) {
        const order = movedFormat(settings, type, step);
        if (order) onChange({ order });
    }

    function setStart(value: string) {
        const codepoint = parseCodepoint(value);
        setStartError(codepoint === undefined ? 'Enter a hexadecimal codepoint, such as F101' : undefined);
        if (codepoint !== undefined) onChange({ startCodepoint: codepoint });
    }

    return (
        <div className="pg-options">
            <div className="pg-options-header">
                <h2>Options</h2>
                <button type="button" className="pg-link" onClick={onReset}>
                    Reset all
                </button>
            </div>

            <details open>
                <summary>Font</summary>
                <label>
                    Font name <input {...text('fontName')} spellCheck={false} />
                </label>
                <label>
                    Font style <input {...text('fontStyle')} placeholder="normal" />
                </label>
                <label>
                    Font weight <input {...text('fontWeight')} placeholder="normal" />
                </label>
            </details>

            <details open>
                <summary>Formats</summary>
                <div className="pg-checks">
                    {ALL_TYPES.map(type => (
                        <label key={type} title={unavailable.includes(type) ? 'Color glyphs need TTF, WOFF, or WOFF2' : undefined}>
                            <input
                                type="checkbox"
                                checked={settings.types.includes(type)}
                                disabled={unavailable.includes(type)}
                                onChange={event => onChange({ types: toggled(settings.types, type, event.target.checked) })}
                            />{' '}
                            {type.toUpperCase()}
                        </label>
                    ))}
                </div>
                <p className="pg-hint">Order in the generated CSS:</p>
                <ol className="pg-order">
                    {ordered.map((type, index) => (
                        <li key={type}>
                            {type.toUpperCase()}
                            <span>
                                <button type="button" disabled={index === 0} aria-label={`Move ${type} up`} onClick={() => move(type, -1)}>
                                    ↑
                                </button>
                                <button type="button" disabled={index === ordered.length - 1} aria-label={`Move ${type} down`} onClick={() => move(type, 1)}>
                                    ↓
                                </button>
                            </span>
                        </li>
                    ))}
                </ol>
                <label>
                    WOFF2 compression quality: {settings.woff2CompressionQuality}
                    <input
                        type="range"
                        min="0"
                        max="11"
                        value={settings.woff2CompressionQuality}
                        onChange={event => onChange({ woff2CompressionQuality: event.target.valueAsNumber })}
                    />
                </label>
            </details>

            <details>
                <summary>Glyph size and alignment</summary>
                <label>
                    Font height <CommitInput {...number('fontHeight')} placeholder="Tallest icon" />
                </label>
                <label>
                    Ascent <CommitInput {...number('ascent')} placeholder="Automatic" />
                </label>
                <label>
                    Descent <CommitInput {...number('descent')} placeholder="0" />
                </label>
                <label>
                    Rounding <CommitInput {...number('round')} placeholder="10e12" />
                </label>
                <label className="pg-toggle">
                    <input {...check('normalize')} /> Normalize to the same height
                </label>
                <label className="pg-toggle">
                    <input {...check('fixedWidth')} /> Fixed width
                </label>
                <label className="pg-toggle">
                    <input {...check('centerHorizontally')} /> Center horizontally
                </label>
                <label className="pg-toggle">
                    <input {...check('centerVertically')} /> Center vertically
                </label>
                <label className="pg-toggle">
                    <input {...check('preserveAspectRatio')} /> Preserve aspect ratio
                </label>
                <label className="pg-toggle">
                    <input {...check('optimizeOutput')} /> Optimize SVG paths
                </label>
            </details>

            <details>
                <summary>Class names and CSS</summary>
                <label>
                    Base selector <input {...text('baseSelector')} spellCheck={false} />
                </label>
                <label>
                    Class prefix <input {...text('classPrefix')} spellCheck={false} />
                </label>
                <label>
                    Font URL prefix <input {...text('cssFontsUrl')} placeholder="Relative to the CSS file" spellCheck={false} />
                </label>
                <label className="pg-toggle">
                    <input {...check('css')} /> Include CSS in downloads
                </label>
                <label className="pg-toggle">
                    <input {...check('html')} /> Include HTML preview in downloads
                </label>
                <label className="pg-toggle">
                    <input {...check('ts')} /> Include TypeScript metadata in downloads
                </label>
                <label>
                    Extra template data (JSON)
                    <textarea {...text('templateData')} rows={3} spellCheck={false} placeholder='{ "brand": "acme" }' />
                </label>
            </details>

            <details>
                <summary>Codepoints and ligatures</summary>
                <label className="pg-toggle">
                    <input {...check('ligature')} /> Ligatures
                </label>
                <label>
                    Start codepoint
                    <CommitInput value={formatCodepoint(settings.startCodepoint)} spellCheck={false} aria-invalid={Boolean(startError)} onCommit={setStart} />
                </label>
                {startError && <small className="pg-options-error">{startError}</small>}
                <p className="pg-hint">New icons get the next free codepoint from here. Edit any icon's codepoint on its card.</p>
                <button type="button" className="pg-button" onClick={onReassign}>
                    Reassign all from the start codepoint
                </button>
            </details>

            <details>
                <summary>Color glyphs</summary>
                <div className="pg-checks">
                    <label>
                        <input {...colorMode('off')} /> Off
                    </label>
                    <label>
                        <input {...colorMode('all')} /> All icons
                    </label>
                    <label>
                        <input {...colorMode('selected')} /> Selected
                    </label>
                </div>
                {settings.colorMode === 'selected' && (
                    <div className="pg-checks pg-color-names">
                        {iconNames.map(name => (
                            <label key={name}>
                                <input
                                    type="checkbox"
                                    checked={settings.colorNames.includes(name)}
                                    onChange={event => onChange({ colorNames: toggled(settings.colorNames, name, event.target.checked) })}
                                />{' '}
                                {name}
                            </label>
                        ))}
                    </div>
                )}
                <p className="pg-hint">Keeps solid SVG colors. Some platforms show the single-color version instead.</p>
            </details>

            <details>
                <summary>Templates</summary>
                {(['cssTemplate', 'htmlTemplate'] as const).map(key => {
                    const template = settings[key];
                    return (
                        <Fragment key={key}>
                            <div className="pg-template">
                                <span>{key === 'cssTemplate' ? 'CSS' : 'HTML'} template</span>
                                {template === undefined ? (
                                    <button type="button" className="pg-link" onClick={() => onChange({ [key]: BUILT_IN_TEMPLATES[key] })}>
                                        Customize
                                    </button>
                                ) : (
                                    <button type="button" className="pg-link" onClick={() => onChange({ [key]: undefined })}>
                                        Use built-in
                                    </button>
                                )}
                            </div>
                            {template !== undefined && (
                                <textarea value={template} rows={10} spellCheck={false} className="pg-code" onChange={event => onChange({ [key]: event.target.value })} />
                            )}
                        </Fragment>
                    );
                })}
            </details>

            <details>
                <summary>Font metadata</summary>
                <label>
                    SVG font ID <input {...text('svgFontId')} placeholder="Font name" />
                </label>
                <label>
                    SVG metadata <input {...text('svgMetadata')} />
                </label>
                <label>
                    TTF copyright <input {...text('ttfCopyright')} />
                </label>
                <label>
                    TTF description <input {...text('ttfDescription')} />
                </label>
                <label>
                    TTF URL <input {...text('ttfUrl')} />
                </label>
                <label>
                    TTF version <input {...text('ttfVersion')} placeholder="1.0" />
                </label>
                <label>
                    TTF timestamp (Unix seconds)
                    <CommitInput {...number('ttfTimestamp')} placeholder="Now" />
                </label>
                <label>
                    WOFF metadata <input {...text('woffMetadata')} />
                </label>
            </details>
        </div>
    );
}
