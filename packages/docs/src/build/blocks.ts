// Components inside Markdown, in the syntax the pages already use: `<ExampleOutput example="basic" />`,
// `:prop="expression"` for values that aren't strings, and `<template #name>` for named slots. A block starts at the
// beginning of a line; its slots are Markdown of their own. Each block becomes a placeholder before markdown-it runs, and
// its rendered HTML takes the placeholder's place afterwards.

export interface Block {
    name: string;
    props: Record<string, unknown>;
    /** Each slot's raw Markdown; the default slot is everything outside `<template #…>`. */
    slots: Record<string, string>;
}

const camel = (name: string) => name.replace(/-(\w)/g, (_, c: string) => c.toUpperCase());
const ENTITIES: Record<string, string> = { '&quot;': '"', '&#39;': "'", '&lt;': '<', '&gt;': '>', '&amp;': '&' };
const decode = (s: string) => s.replace(/&(?:quot|#39|lt|gt|amp);/g, e => ENTITIES[e]!);

/** Reads attributes from `at` to the end of the opening tag; returns them and the index after `>` or `/>`. */
function readTag(source: string, at: number): { props: Record<string, unknown>; end: number; selfClosing: boolean } {
    const props: Record<string, unknown> = {};
    const attr = /\s*(?:(\/?>)|([:@]?[\w-]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'))?)/y;
    attr.lastIndex = at;
    for (let m = attr.exec(source); m; m = attr.exec(source)) {
        if (m[1]) return { props, end: attr.lastIndex, selfClosing: m[1] === '/>' };
        const raw = m[3] ?? m[4];
        const name = m[2]!;
        if (name.startsWith(':')) {
            // Values are trusted docs source, evaluated once at build time, as Vue would in the browser.
            // oxlint-disable-next-line no-implied-eval
            props[camel(name.slice(1))] = new Function(`return (${decode(raw ?? '')});`)() as unknown;
        } else props[camel(name)] = raw === undefined ? true : decode(raw);
    }
    throw new Error(`Unclosed tag at ${source.slice(at - 20, at + 40)}`);
}

/** The index of the `</name>` that closes a block opened before `from`, skipping nested blocks of the same name. */
function closing(source: string, name: string, from: number): number {
    const tags = new RegExp(`<(/?)${name}\\b`, 'g');
    tags.lastIndex = from;
    let depth = 1;
    for (let m = tags.exec(source); m; m = tags.exec(source)) {
        if (!m[1]) {
            if (!readTag(source, tags.lastIndex).selfClosing) depth++;
        } else if (--depth === 0) return m.index;
    }
    throw new Error(`<${name}> is never closed`);
}

/** The fence a line opens or closes (` ``` ` or `~~~`, three or more), if any. */
const fenceMarker = (line: string) => /^\s*(`{3,}|~{3,})/.exec(line)?.[1];
const closesFence = (line: string, fence: string) => {
    const marker = fenceMarker(line);
    return !!marker && marker[0] === fence[0] && marker.length >= fence.length && !line.trim().slice(marker.length);
};

/**
 * Splits a block's body into its `<template #name>` slots and the default slot. Tags inside fenced code are code: a
 * snippet of a Vue file has a `</template>` of its own.
 */
function slotsOf(body: string): Record<string, string> {
    const slots: Record<string, string> = {};
    const rest: string[] = [];
    let slot: { name: string; lines: string[]; depth: number } | undefined;
    let fence: string | undefined;
    for (const line of body.split('\n')) {
        const target = slot?.lines ?? rest;
        if (fence) {
            if (closesFence(line, fence)) fence = undefined;
            target.push(line);
            continue;
        }
        fence = fenceMarker(line);
        if (fence) {
            target.push(line);
            continue;
        }
        const open = /^<template #([\w-]+)>\s*$/.exec(line);
        if (!slot && open) {
            slot = { name: open[1]!, lines: [], depth: 0 };
            continue;
        }
        if (slot && /^<\/template>\s*$/.test(line) && slot.depth-- === 0) {
            slots[slot.name] = slot.lines.join('\n').replace(/^\n/, '').replace(/\n$/, '');
            slot = undefined;
            continue;
        }
        if (slot && /^<template(?:\s[^>]*)?(?<!\/)>\s*$/.test(line)) slot.depth++;
        target.push(line);
    }
    if (slot) throw new Error(`<template #${slot.name}> is never closed`);
    const fallback = rest.join('\n').trim();
    if (fallback) slots.default = fallback;
    return slots;
}

/**
 * Replaces every top-level block of a known component with `placeholder(i)`, returning the blocks in order. Fenced code is
 * left alone, so code samples can show the same syntax.
 */
export function extractBlocks(source: string, names: Set<string>, placeholder: (i: number) => string): { source: string; blocks: Block[] } {
    const blocks: Block[] = [];
    let out = '';
    let i = 0;
    let fence: string | undefined;
    const line = /^.*$/gm;
    while (i < source.length) {
        line.lastIndex = i;
        const text = line.exec(source)![0];
        const next = i + text.length + 1;
        if (fence) {
            if (closesFence(text, fence)) fence = undefined;
        } else fence = fenceMarker(text);
        const name = !fence && /^<([A-Z]\w*)\b/.exec(text)?.[1];
        if (!name || !names.has(name)) {
            out += source.slice(i, Math.min(next, source.length));
            i = next;
            continue;
        }
        const tag = readTag(source, i + name.length + 1);
        let end = tag.end;
        let body = '';
        if (!tag.selfClosing) {
            const close = closing(source, name, tag.end);
            body = source.slice(tag.end, close).replace(/^\n/, '').replace(/\n$/, '');
            end = close + name.length + 3;
        }
        blocks.push({ name, props: tag.props, slots: slotsOf(body) });
        out += `\n${placeholder(blocks.length - 1)}\n`;
        i = end;
    }
    return { source: out, blocks };
}
