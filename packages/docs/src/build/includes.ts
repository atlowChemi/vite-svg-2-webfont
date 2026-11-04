import { readFileSync } from 'node:fs';
import { dirname, extname, resolve } from 'node:path';

// VitePress's source-level includes, resolved before parsing so every consumer (pages, search, llms.txt) sees the same text:
// `<!--@include: path-->` splices another Markdown file in (`path{2,}` only some of its lines), and `<<< path` turns a source file into a code block.
// `@/` is the docs root; anything else is relative to the including file.

const INCLUDE = /^[ \t]*<!--\s*@include:\s*([^\s{]+?)(?:\{(\d*),(\d*)\})?\s*-->[ \t]*$/gm;
const SNIPPET = /^[ \t]*<<<\s+([^\s{]+)(?:\{([^}]*)\})?(?:\s+\[([^\]]+)\])?[ \t]*$/gm;

export function expandIncludes(source: string, file: string, root: string, deps: Set<string>, depth = 0): string {
    if (depth > 8) throw new Error(`Include depth exceeded in ${file}`);
    const at = (path: string) => (path.startsWith('@/') ? resolve(root, path.slice(2)) : resolve(dirname(file), path));
    return source
        .replace(INCLUDE, (_, path: string, from?: string, to?: string) => {
            const target = at(path);
            deps.add(target);
            // Included fragments carry no frontmatter of their own that should survive. `{2,}` keeps lines 2 to the end
            // (1-based, inclusive), e.g. a changelog without its own title.
            let text = readFileSync(target, 'utf8').replace(/^---\n[\s\S]*?\n---\n/, '');
            if (from !== undefined)
                text = text
                    .split('\n')
                    .slice(Number(from || 1) - 1, to ? Number(to) : undefined)
                    .join('\n');
            return expandIncludes(text, target, root, deps, depth + 1).trimEnd();
        })
        .replace(SNIPPET, (_, path: string, meta = '', title?: string) => {
            const target = at(path);
            deps.add(target);
            const code = readFileSync(target, 'utf8').trimEnd();
            const fence = code.includes('```') ? '````' : '```';
            // VitePress titles a snippet with its file name; the language comes from the extension.
            // `{1-3 lang}` after the path: highlighted lines, then an optional language override.
            const [lines = '', lang = extname(target).slice(1)] =
                /^([\d,\s-]*)\s*(\w*)$/
                    .exec(meta.trim())
                    ?.slice(1)
                    .map(s => s.trim() || undefined) ?? [];
            const name = title ?? target.slice(target.lastIndexOf('/') + 1);
            return `${fence}${lang}${lines ? `{${lines}}` : ''} [${name}]\n${code}\n${fence}`;
        });
}
