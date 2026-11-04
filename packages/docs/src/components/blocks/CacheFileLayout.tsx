// The persistent cache file's layout, top to bottom, as the engine writes it (crates/webfont-generator/src/cache).
interface Field {
    name: string;
    size: string;
}
interface Block {
    title: string;
    note: string;
    fields: Field[];
    repeat?: { label: string; fields: Field[] };
}

const blocks: Block[] = [
    {
        title: 'Header',
        note: 'Any mismatch and the whole file is ignored.',
        fields: [
            { name: 'Magic: WFGCACHE', size: '8 bytes' },
            { name: 'Format version', size: '4 bytes' },
            { name: 'Engine version', size: 'text' },
            { name: 'Settings fingerprint', size: '16 bytes' },
        ],
    },
    {
        title: 'Per-icon work',
        note: 'Read only when an icon or format must be rebuilt.',
        fields: [
            { name: 'CRC32', size: '4 bytes' },
            { name: 'Length', size: '4 bytes' },
        ],
        repeat: {
            label: 'For each SVG, by content hash',
            fields: [
                { name: 'Parsed outlines and color layers', size: 'paths' },
                { name: 'Processed glyph', size: 'paths' },
            ],
        },
    },
    {
        title: 'Finished fonts',
        note: 'Checked on every load. Up to two entries per format.',
        fields: [
            { name: 'CRC32', size: '4 bytes' },
            { name: 'Length', size: '4 bytes' },
        ],
        repeat: {
            label: 'For each font',
            fields: [
                { name: 'Format', size: '1 byte' },
                { name: 'Key: format options and icons', size: '16 bytes' },
                { name: 'Icon set: names, codepoints, contents', size: '16 bytes' },
                { name: 'Font bytes', size: 'bytes' },
            ],
        },
    },
];

function Fields({ fields }: { fields: Field[] }) {
    return fields.map(field => (
        <div key={field.name} className="field">
            <span>{field.name}</span>
            <code>{field.size}</code>
        </div>
    ));
}

/** The `.wfcache` file's sections and fields, drawn as stacked blocks. */
export function CacheFileLayout() {
    return (
        <figure className="cache-layout" aria-label="Layout of a .wfcache file">
            {blocks.map(block => (
                <div key={block.title} className="block">
                    <div className="head">
                        <span className="title">{block.title}</span>
                        <span className="note">{block.note}</span>
                    </div>
                    <div className="fields">
                        <Fields fields={block.fields} />
                        {block.repeat && (
                            <div className="repeat">
                                <span className="label">{block.repeat.label}</span>
                                <Fields fields={block.repeat.fields} />
                            </div>
                        )}
                    </div>
                </div>
            ))}
            <figcaption>
                <code>icons.3f2a9c41d0b7e8a5.wfcache</code>, little-endian, top to bottom
            </figcaption>
        </figure>
    );
}
