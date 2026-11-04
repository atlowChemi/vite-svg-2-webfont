import { Html } from './Html.tsx';

const ENTITIES: Record<string, string> = { '&quot;': '"', '&#x27;': "'", '&#39;': "'", '&lt;': '<', '&gt;': '>', '&amp;': '&' };
const lineText = (line: string) => line.replaceAll(/<[^>]+>/g, '').replaceAll(/&(?:quot|#x27|#39|lt|gt|amp);/g, entity => ENTITIES[entity]!);

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
