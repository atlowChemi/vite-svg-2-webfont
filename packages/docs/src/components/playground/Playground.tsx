import { useState } from 'react';
import { PlaygroundDropZone } from './PlaygroundDropZone.tsx';
import { PlaygroundIconCard } from './PlaygroundIconCard.tsx';
import { PlaygroundOptions } from './PlaygroundOptions.tsx';
import { PlaygroundOutputs } from './PlaygroundOutputs.tsx';
import { PlaygroundToast } from './PlaygroundToast.tsx';
import { readSvgFiles } from './files.ts';
import { useIconList } from './useIconList.ts';
import { useNotice } from './useNotice.ts';
import { useStoredSettings } from './useStoredSettings.ts';
import { useWebfontBuild } from './useWebfontBuild.ts';

const samples = import.meta.glob<string>('../../../examples/icons/*.svg', { query: '?raw', import: 'default', eager: true });
const sampleSources = Object.entries(samples).map(([path, svg]) => ({ fileName: path.split('/').at(-1)!, svg }));

/** The Playground: drop SVGs, tune the options, preview the font and download it. Runs only in the browser. */
export function Playground() {
    const { settings, update: updateSettings, reset: resetSettings } = useStoredSettings();
    const { icons, errors, addSources, rename, setCodepoint, remove, removeAll, reassign } = useIconList(settings, updateSettings);
    const { build, error, building } = useWebfontBuild(icons, settings);
    const { notice, notify, dismiss } = useNotice();
    const [size, setSize] = useState(40);

    async function addFiles(files: Iterable<File>) {
        const { sources, skipped } = await readSvgFiles(files);
        if (skipped) notify(skipped);
        addSources(sources);
    }

    return (
        <div className="pg">
            <PlaygroundToast message={notice} onDismiss={dismiss} />
            <PlaygroundDropZone onFiles={files => void addFiles(files)} onSamples={() => addSources(sampleSources)} />

            <div className="pg-layout">
                <section className="pg-main">
                    <div className="pg-toolbar">
                        <h2>Preview</h2>
                        <span className="pg-status" role="status">
                            {building ? 'Building…' : build ? `Built in ${build.milliseconds} ms` : null}
                        </span>
                        <label className="pg-size">
                            Size <input type="range" min="16" max="96" value={size} onChange={event => setSize(event.target.valueAsNumber)} />
                        </label>
                        {icons.length > 0 && (
                            <button type="button" className="pg-link" onClick={removeAll}>
                                Remove all
                            </button>
                        )}
                    </div>

                    {error && (
                        <p className="pg-error" role="alert">
                            {error}
                        </p>
                    )}
                    {!icons.length ? (
                        <p className="pg-empty">Add some SVG icons to see them as a font.</p>
                    ) : (
                        build && !build.previewFamily && <p className="pg-hint">Select WOFF2, WOFF, or TTF to preview the font. The cards show the source SVGs meanwhile.</p>
                    )}

                    <div className="pg-grid">
                        {icons.map(icon => (
                            <PlaygroundIconCard
                                key={icon.id}
                                icon={icon}
                                previewFamily={build?.previewFamily}
                                built={build?.codepoints.has(icon.codepoint)}
                                showSource={Boolean(build) && !build?.previewFamily}
                                size={size}
                                classPrefix={settings.classPrefix}
                                nameError={errors[icon.id]?.name}
                                codepointError={errors[icon.id]?.codepoint}
                                onRename={name => rename(icon, name)}
                                onCodepoint={text => setCodepoint(icon, text)}
                                onRemove={() => remove(icon)}
                            />
                        ))}
                    </div>

                    {build && <PlaygroundOutputs build={build} settings={settings} icons={icons} />}
                </section>

                <aside className="pg-aside">
                    <PlaygroundOptions settings={settings} iconNames={icons.map(icon => icon.name)} onChange={updateSettings} onReassign={reassign} onReset={resetSettings} />
                </aside>
            </div>
        </div>
    );
}
