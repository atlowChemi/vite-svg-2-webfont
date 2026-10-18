import { describe, expect, it } from 'vite-plus/test';
import { isSvgMarkup, readSvgFiles, skippedFilesMessage } from './files';

const file = (name: string, content: string, type = '') => new File([content], name, { type });

describe('files', () => {
    it('reads SVGs by extension or media type', async () => {
        const { sources, skipped } = await readSvgFiles([file('add.SVG', '<svg></svg>'), file('icon', '<svg viewBox="0 0 1 1"></svg>', 'image/svg+xml')]);
        expect(sources).toEqual([
            { fileName: 'add.SVG', svg: '<svg></svg>' },
            { fileName: 'icon', svg: '<svg viewBox="0 0 1 1"></svg>' },
        ]);
        expect(skipped).toBeUndefined();
    });

    it('explains files that are not SVGs or have no SVG markup', async () => {
        const { sources, skipped } = await readSvgFiles([file('ok.svg', '<svg></svg>'), file('photo.png', 'png'), file('broken.svg', 'not markup')]);
        expect(sources.map(source => source.fileName)).toEqual(['ok.svg']);
        expect(skipped).toBe('Skipped 2 files that is not an SVG: photo.png; has no SVG content: broken.svg.');
    });

    it('words single and multiple skipped files', () => {
        expect(skippedFilesMessage(['photo.png'], [])).toBe('Skipped 1 file that is not an SVG: photo.png.');
        expect(skippedFilesMessage([], ['a.svg', 'b.svg'])).toBe('Skipped 2 files that have no SVG content: a.svg, b.svg.');
        expect(skippedFilesMessage([], [])).toBeUndefined();
    });

    it('names the first three skipped files and counts the rest', () => {
        expect(skippedFilesMessage(['1.png', '2.png', '3.png', '4.png', '5.png'], [])).toBe('Skipped 5 files that are not SVGs: 1.png, 2.png, 3.png, and 2 more.');
    });

    it('requires an svg element, not just the letters', () => {
        expect(isSvgMarkup('<?xml version="1.0"?>\n<SVG\nwidth="1">')).toBe(true);
        expect(isSvgMarkup('<svgfoo>')).toBe(false);
    });
});
