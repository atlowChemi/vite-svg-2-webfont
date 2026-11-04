import { Html } from './Html.tsx';

const ENTITIES: Record<string, string> = { '&quot;': '"', '&#x27;': "'", '&#39;': "'", '&lt;': '<', '&gt;': '>', '&amp;': '&' };
/** A highlighted line's text, for matching snippets: its tags removed (repeatedly, so none can re-form) and entities decoded. */
function lineText(line: string): string {
    let previous;
    let text = line;
    do {
        previous = text;
        text = text.replaceAll(/<[^>]+>/g, '');
    } while (text !== previous);
    return text.replaceAll(/&(?:quot|#x27|#39|lt|gt|amp);/g, entity => ENTITIES[entity]!);
}

/**
 * The CSS the plugin generated for one example, highlighted at build time (`html`). Lines containing any `highlight`
 * snippet get the highlighted-line style, matching the config tab's highlights.
 */
export function ExampleCss({ html, highlight = [] }: { html: string; highlight?: string[] }) {
    const marked = highlight.length
        ? html
              .split('\n')
              .map(line =>
                  line.includes('<span class="line">') && highlight.some(snippet => lineText(line).includes(snippet))
                      ? line.replace('<span class="line">', '<span class="line highlighted">')
                      : line,
              )
              .join('\n')
        : html;
    return <Html className="example-css" html={marked} />;
}
