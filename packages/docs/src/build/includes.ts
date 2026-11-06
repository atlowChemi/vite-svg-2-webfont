import { readFile } from 'node:fs/promises';
import { basename, dirname, extname, resolve } from 'node:path';

// VitePress's source-level includes, resolved before parsing so every consumer (pages, search, llms.txt) sees the same text:
// `<!--@include: path-->` splices another Markdown file in (`path{2,}` only some of its lines), and `<<< path` turns a source file into a code block.
// `@/` is the docs' `src/`; anything else is relative to the including file.

// Matches Markdown includes and code snippets in a single pass.
const DIRECTIVE = /^[ \t]*(?:<!--\s*@include:\s*([^\s{]+?)(?:\{(\d*),(\d*)\})?\s*-->|<<<\s+([^\s{]+)(?:\{([^}]*)\})?(?:\s+\[([^\]]+)\])?)[ \t]*$/gm;

/** Matches frontmatter header and groups the contents. */
export const frontmatterMatcher = /^---\n([\s\S]*?)\n---\n/;

/**
 * Expands VitePress-style Markdown includes and code snippets.
 * Tracks referenced files and recursively resolves nested includes.
 */
export async function expandIncludes(source: string, file: string, root: string, deps: Set<string>, depth = 0): Promise<string> {
    if (depth > 8) throw new Error(`Include depth exceeded in ${file}`);

    // Resolve '@/...' from the docs root, other paths from the current file.
    const at = (path: string) => (path.startsWith('@/') ? resolve(root, path.slice(2)) : resolve(dirname(file), path));

    // Expand independent directives concurrently, preserving their positions.
    const replacements = await Promise.all(
        Array.from(source.matchAll(DIRECTIVE), async match => {
            const [original, includePath, from, to, snippetPath, meta = '', title] = match;
            const target = at(includePath ?? snippetPath!);
            deps.add(target);

            let text = await readFile(target, 'utf8');

            if (includePath !== undefined) {
                // Strip frontmatter from included Markdown.
                text = text.replace(frontmatterMatcher, '');

                // Select an optional 1-based, inclusive line range.
                if (from !== undefined) {
                    text = text
                        .split('\n')
                        .slice(Number(from || 1) - 1, to ? Number(to) : undefined)
                        .join('\n');
                }

                // Expand nested directives before inserting the fragment.
                text = await expandIncludes(text, target, root, deps, depth + 1);
                return { index: match.index, length: original.length, text: text.trimEnd() };
            }

            // Convert source files into fenced Markdown code blocks.
            const code = text.trimEnd();

            // Use a fence longer than any backtick sequence in the code.
            let fenceLength = 3;
            for (const currentFence of code.matchAll(/`+/g)) {
                fenceLength = Math.max(fenceLength, currentFence[0].length + 1);
            }
            const fence = '`'.repeat(fenceLength);

            // Parse highlighted lines and an optional language override.
            const [lines = '', lang = extname(target).slice(1)] =
                /^([\d,\s-]*)\s*(\w*)$/
                    .exec(meta.trim())
                    ?.slice(1)
                    .map(s => s.trim() || undefined) ?? [];

            const name = title ?? basename(target);
            return {
                index: match.index,
                length: original.length,
                text: `${fence}${lang}${lines ? `{${lines}}` : ''} [${name}]\n${code}\n${fence}`,
            };
        }),
    );

    // Reconstruct the source without allocating intermediate full strings.
    const parts: string[] = [];
    let cursor = 0;

    for (const { index, length, text } of replacements) {
        parts.push(source.slice(cursor, index), text);
        cursor = index + length;
    }

    parts.push(source.slice(cursor));
    return parts.join('');
}
