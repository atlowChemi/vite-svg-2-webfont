/**
 * Parses components embedded in Markdown, in the syntax the pages already use:
 *
 * - `<ExampleOutput example="basic" />`: a block starts at the beginning of a line.
 * - `:prop="expression"` for values that aren't strings; `prop` alone is `true`.
 * - `<template #name>` for named slots; the rest of the body is the default slot. Slots are Markdown of their own.
 *
 * Supports nested components, and ignores component syntax inside fenced code, so code samples can show it.
 *
 * Extracted components are replaced with placeholders before Markdown
 * rendering, then restored as rendered HTML afterwards.
 */

export interface Block<T extends string> {
    name: T;
    props: Record<string, unknown>;
    /** Raw Markdown for each slot; 'default' contains content outside named slots. */
    slots: Record<string, string>;
}

const camel = (name: string) => name.replace(/-(\w)/g, (_, c: string) => c.toUpperCase());

const ENTITIES: Record<string, string> = {
    '&quot;': '"',
    '&#39;': "'",
    '&lt;': '<',
    '&gt;': '>',
    '&amp;': '&',
};

const decode = (s: string) => s.replace(/&(?:quot|#39|lt|gt|amp);/g, entity => ENTITIES[entity]!);

/**
 * Returns the opening Markdown fence marker (three or more backticks or tildes).
 * Any indentation counts: a fence inside a list item is indented past CommonMark's top-level three spaces.
 */
const fenceMarker = (line: string): string | undefined => /^[ \t]*(`{3,}|~{3,})/.exec(line)?.[1];

/** Checks whether a line closes an existing Markdown code fence. */
const closesFence = (line: string, fence: string): boolean => {
    const marker = /^[ \t]*(`+|~+)[ \t]*$/.exec(line)?.[1];
    return !!marker && marker[0] === fence[0] && marker.length >= fence.length;
};

/**
 * Reads an opening component tag, including its attributes.
 * Returns its props, whether it is self-closing, and the position after '>'.
 *
 * Bound props (:name="expression") are evaluated at build time.
 * Only use this parser with trusted documentation sources.
 */
function readTag(source: string, at: number): { props: Record<string, unknown>; end: number; selfClosing: boolean } {
    const props: Record<string, unknown> = {};
    const attr = /\s*(?:(\/?>)|([:@]?[\w-]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'))?)/y;

    attr.lastIndex = at;

    for (let match = attr.exec(source); match; match = attr.exec(source)) {
        if (match[1]) {
            return {
                props,
                end: attr.lastIndex,
                selfClosing: match[1] === '/>',
            };
        }

        const name = match[2]!;
        const raw = match[3] ?? match[4];

        if (name.startsWith(':')) {
            // Evaluate trusted Vue-style bound expressions once at build time.
            // oxlint-disable-next-line no-implied-eval
            props[camel(name.slice(1))] = new Function(`return (${decode(raw ?? '')});`)() as unknown;
        } else {
            // Attributes without values are boolean; others are decoded strings.
            props[camel(name)] = raw === undefined ? true : decode(raw);
        }
    }

    throw new Error(`Unclosed tag at ${source.slice(at - 20, at + 40)}`);
}

/**
 * Finds the closing tag matching a component opened before 'from'.
 * Accounts for nested components of the same name, self-closing tags,
 * and fenced code blocks.
 */
function closing(source: string, name: string, from: number): number {
    // Any component tag; ones with another name are skipped. A nested opening tag is read whole, so a `<` or `>` in its
    // attribute values can't be mistaken for a tag.
    const tag = /<\/?([A-Z]\w*)\b/g;
    let depth = 1;
    let fence: string | undefined;
    let i = from;

    while (i < source.length) {
        const newline = source.indexOf('\n', i);
        const end = newline === -1 ? source.length : newline;
        const line = source.slice(i, end);
        const next = newline === -1 ? end : end + 1;

        if (fence) {
            if (closesFence(line, fence)) fence = undefined;
            i = next;
            continue;
        }

        if ((fence = fenceMarker(line))) {
            i = next;
            continue;
        }

        tag.lastIndex = i;

        for (let match = tag.exec(source); match && match.index < end; match = tag.exec(source)) {
            if (match[1] !== name) continue;

            const closingTag = source[match.index + 1] === '/';

            if (closingTag) {
                if (--depth === 0) return match.index;
            } else {
                const parsed = readTag(source, tag.lastIndex);
                if (!parsed.selfClosing) depth++;
                tag.lastIndex = parsed.end;
            }
        }

        i = next;
    }

    throw new Error(`<${name}> is never closed`);
}

/**
 * Splits a component body into named <template #name> slots and a default slot.
 * Ignores template syntax inside fenced code and tracks nested templates.
 */
function slotsOf(body: string): Record<string, string> {
    const slots: Record<string, string> = {};
    const rest: string[] = [];

    let slot: { name: string; lines: string[]; depth: number } | undefined;
    let fence: string | undefined;

    // Scan lazily rather than allocating an array with body.split('\n').
    for (let i = 0; i <= body.length;) {
        const newline = body.indexOf('\n', i);
        const end = newline === -1 ? body.length : newline;
        const line = body.slice(i, end);
        i = newline === -1 ? body.length + 1 : newline + 1;

        const target = slot?.lines ?? rest;

        // Treat all content inside fenced code as literal Markdown.
        if (fence) {
            if (closesFence(line, fence)) fence = undefined;
            target.push(line);
            continue;
        }

        if ((fence = fenceMarker(line))) {
            target.push(line);
            continue;
        }

        // A top-level named template begins a new slot.
        const open = /^<template #([\w-]+)>\s*$/.exec(line);
        if (!slot && open) {
            slot = { name: open[1]!, lines: [], depth: 0 };
            continue;
        }

        if (slot) {
            // Nested templates must close before the named slot can close.
            if (/^<\/template>\s*$/.test(line)) {
                if (slot.depth === 0) {
                    slots[slot.name] = slot.lines.join('\n').replace(/^\n|\n$/g, '');
                    slot = undefined;
                    continue;
                }

                slot.depth--;
            } else if (/^<template(?:\s[^>]*)?(?<!\/)>\s*$/.test(line)) {
                slot.depth++;
            }
        }

        target.push(line);
    }

    if (slot) throw new Error(`<template #${slot.name}> is never closed`);

    const fallback = rest.join('\n').trim();
    if (fallback) slots.default = fallback;

    return slots;
}

/**
 * Extracts top-level blocks of known components from Markdown.
 *
 * Replaces each block with placeholder(index), returning the modified
 * source and extracted blocks in their original order.
 *
 * Component-like syntax inside fenced code is ignored.
 */
export function extractBlocks<T extends string>(source: string, names: Set<T>, placeholder: (i: number) => string): { source: string; blocks: Block<T>[] } {
    const blocks: Block<T>[] = [];
    const parts: string[] = [];

    let i = 0;
    let last = 0;
    let fence: string | undefined;

    while (i < source.length) {
        // Scan one line at a time without allocating a split array.
        const newline = source.indexOf('\n', i);
        const end = newline === -1 ? source.length : newline;
        const line = source.slice(i, end);
        const next = newline === -1 ? end : end + 1;

        // Ignore component syntax inside fenced Markdown code blocks.
        if (fence) {
            if (closesFence(line, fence)) fence = undefined;
            i = next;
            continue;
        }

        if ((fence = fenceMarker(line))) {
            i = next;
            continue;
        }

        // Components must begin at column zero and have a known name.
        const name = line[0] === '<' ? (/^<([A-Z]\w*)\b/.exec(line)?.[1] as T | undefined) : undefined;

        if (!name || !names.has(name)) {
            i = next;
            continue;
        }

        // Parse the opening tag and find the end of the component block.
        const tag = readTag(source, i + name.length + 1);
        let blockEnd = tag.end;
        let body = '';

        if (!tag.selfClosing) {
            const close = closing(source, name, tag.end);
            body = source.slice(tag.end, close).replace(/^\n|\n$/g, '');
            blockEnd = close + name.length + 3;
        }

        // Save the component and replace its source with a placeholder.
        blocks.push({ name, props: tag.props, slots: slotsOf(body) });
        parts.push(source.slice(last, i), `\n${placeholder(blocks.length - 1)}\n`);

        // Skip the extracted component, retaining surrounding text verbatim.
        i = last = blockEnd;
    }

    // Append the remaining source after the final extracted block.
    parts.push(source.slice(last));

    return { source: parts.join(''), blocks };
}
