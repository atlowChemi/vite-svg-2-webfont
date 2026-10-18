export type SvgSourceFile = { fileName: string; svg: string };

/** Whether text contains SVG markup; a renamed or corrupt file can carry an .svg name without it. */
export function isSvgMarkup(text: string): boolean {
    return /<svg[\s>]/i.test(text);
}

function listFiles(names: string[]): string {
    const shown = names.slice(0, 3).join(', ');
    return names.length > 3 ? `${shown}, and ${names.length - 3} more` : shown;
}

/** A sentence naming skipped files, or undefined when nothing was skipped. */
export function skippedFilesMessage(notSvg: string[], invalid: string[]): string | undefined {
    const reasons = [
        notSvg.length && `${notSvg.length === 1 ? 'is not an SVG' : 'are not SVGs'}: ${listFiles(notSvg)}`,
        invalid.length && `${invalid.length === 1 ? 'has' : 'have'} no SVG content: ${listFiles(invalid)}`,
    ].filter(Boolean);
    const total = notSvg.length + invalid.length;
    return reasons.length ? `Skipped ${total} ${total === 1 ? 'file' : 'files'} that ${reasons.join('; ')}.` : undefined;
}

function isSvgFile(file: File): boolean {
    return file.type === 'image/svg+xml' || file.name.toLowerCase().endsWith('.svg');
}

/** Read the SVGs among dropped or picked files, and describe any that were skipped. */
export async function readSvgFiles(files: Iterable<File>): Promise<{ sources: SvgSourceFile[]; skipped?: string }> {
    const { named = [], notSvg = [] } = Object.groupBy(files, file => (isSvgFile(file) ? 'named' : 'notSvg'));
    const read = await Promise.all(named.map(async file => ({ fileName: file.name, svg: await file.text() })));
    const { sources = [], invalid = [] } = Object.groupBy(read, source => (isSvgMarkup(source.svg) ? 'sources' : 'invalid'));
    return {
        sources,
        skipped: skippedFilesMessage(
            notSvg.map(file => file.name),
            invalid.map(source => source.fileName),
        ),
    };
}
