const formats = ['woff2', 'woff', 'ttf', 'eot', 'svg'];

/** What a cast produces: the files beside a sample of them (`snippet`, the rendered `outputs` slot of index.md). */
export function Outputs({ snippet }: { snippet: string }) {
    return (
        <div className="landing-files">
            <div>
                <div className="landing-file">
                    <span className="ext mono">fonts</span>
                    <span className="what chips mono">
                        {formats.map(format => (
                            <span key={format} className="chip">
                                {format}
                            </span>
                        ))}
                    </span>
                </div>
                <OutputFile ext=".css .scss" what="A class for every glyph, or your own template" />
                <OutputFile ext=".html" what="A preview page of the whole set" />
                <OutputFile ext=".ts" what="Icon names and a union type, for type-safe components" />
            </div>
            <div className="landing-snippet vp-doc" dangerouslySetInnerHTML={{ __html: snippet }} />
        </div>
    );
}

function OutputFile({ ext, what }: { ext: string; what: string }) {
    return (
        <div className="landing-file">
            <span className="ext mono">{ext}</span>
            <span className="what">{what}</span>
        </div>
    );
}
