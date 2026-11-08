import { describe, expect, it } from 'vite-plus/test';
import { addSources, emptyIconList, reassignCodepoints, removeIcon, renameIcon, setIconCodepoint, type IconList } from './iconList.ts';
import { DEFAULT_START_CODEPOINT } from './codepoints.ts';

const svg = (label: string) => `<svg><title>${label}</title></svg>`;
const sources = (...names: string[]) => names.map(name => ({ fileName: `${name}.svg`, svg: svg(name) }));
const add = (list: IconList, ...names: string[]) => addSources(list, sources(...names), DEFAULT_START_CODEPOINT);

describe('iconList', () => {
    it('names icons after their files and numbers them from the start codepoint', () => {
        const { icons } = addSources(
            emptyIconList,
            [
                { fileName: 'add.svg', svg: svg('a') },
                { fileName: 'Arrow Left.svg', svg: svg('b') },
            ],
            0xe001,
        );
        expect(icons.map(({ name, codepoint }) => ({ name, codepoint }))).toEqual([
            { name: 'add', codepoint: 0xe001 },
            { name: 'Arrow-Left', codepoint: 0xe002 },
        ]);
    });

    it('replaces the drawing of a same-named icon and keeps its codepoint', () => {
        let list = addSources(emptyIconList, [{ fileName: 'add.svg', svg: svg('old') }], DEFAULT_START_CODEPOINT);
        list = setIconCodepoint(list, list.icons[0]!.id, 'e123');
        list = addSources(list, [{ fileName: 'add.svg', svg: svg('new') }], DEFAULT_START_CODEPOINT);
        expect(list.icons).toHaveLength(1);
        expect(list.icons[0]).toMatchObject({ name: 'add', svg: svg('new'), codepoint: 0xe123 });
    });

    it('keeps the same list when there is nothing to add', () => {
        const list = add(emptyIconList, 'a');
        expect(addSources(list, [], DEFAULT_START_CODEPOINT)).toBe(list);
    });

    it('never changes the list it is given', () => {
        const list = add(emptyIconList, 'a');
        const snapshot = structuredClone(list);
        add(list, 'b', 'a');
        renameIcon(list, [], list.icons[0]!.id, 'star');
        setIconCodepoint(list, list.icons[0]!.id, 'e001');
        removeIcon(list, [], list.icons[0]!.id);
        reassignCodepoints(list, 0xe000);
        expect(list).toEqual(snapshot);
    });

    it('skips codepoints already in use', () => {
        let list = add(emptyIconList, 'a');
        list = setIconCodepoint(list, list.icons[0]!.id, 'f102');
        list = add(list, 'b', 'c');
        expect(list.icons.map(icon => icon.codepoint)).toEqual([0xf102, 0xf101, 0xf103]);
    });

    it('rejects invalid and duplicate names, and carries color selections across renames', () => {
        const list = add(emptyIconList, 'a', 'b');
        const [a, b] = list.icons as [IconList['icons'][0], IconList['icons'][0]];

        let next = renameIcon(list, ['a'], a.id, 'my icon');
        expect(next.list.errors[a.id]?.name).toBe('Use letters, numbers, - and _ only. Falling back to a.');
        next = renameIcon(next.list, next.colorNames, a.id, 'b');
        expect(next.list.errors[a.id]?.name).toBe('Another icon has this name. Falling back to a.');
        expect(next.list.icons.map(icon => icon.name)).toEqual(['a', 'b']);
        expect(next.colorNames).toEqual(['a']);

        next = renameIcon(next.list, next.colorNames, a.id, 'star');
        expect(next.list.errors[a.id]?.name).toBeUndefined();
        expect(next.list.icons.map(icon => icon.name)).toEqual(['star', 'b']);
        expect(next.colorNames).toEqual(['star']);
        expect(next.list.icons[1]).toBe(b);
    });

    it('keeps the same color selection when the renamed icon is not selected', () => {
        const list = add(emptyIconList, 'a', 'b');
        const colorNames = ['b'];
        expect(renameIcon(list, colorNames, list.icons[0]!.id, 'star').colorNames).toBe(colorNames);
    });

    it('rejects unparsable and duplicate codepoints', () => {
        let list = add(emptyIconList, 'a', 'b');
        const { id } = list.icons[1]!;

        list = setIconCodepoint(list, id, 'zz');
        expect(list.errors[id]?.codepoint).toBe('Enter a hexadecimal codepoint, such as F101. Falling back to U+F102.');
        list = setIconCodepoint(list, id, 'F101');
        expect(list.errors[id]?.codepoint).toBe('Another icon uses this codepoint. Falling back to U+F102.');
        expect(list.icons[1]!.codepoint).toBe(0xf102);

        list = setIconCodepoint(list, id, 'U+E001');
        expect(list.errors[id]?.codepoint).toBeUndefined();
        expect(list.icons[1]!.codepoint).toBe(0xe001);
    });

    it('drops removed icons from the color selection', () => {
        const list = add(emptyIconList, 'a', 'b');
        const next = removeIcon(list, ['a', 'b'], list.icons[0]!.id);
        expect(next.list.icons.map(icon => icon.name)).toEqual(['b']);
        expect(next.colorNames).toEqual(['b']);
    });

    it('renumbers every icon in list order and clears errors', () => {
        let list = add(emptyIconList, 'a', 'b');
        list = setIconCodepoint(list, list.icons[0]!.id, 'e050');
        list = setIconCodepoint(list, list.icons[1]!.id, 'nope');

        list = reassignCodepoints(list, 0xe000);
        expect(list.icons.map(icon => icon.codepoint)).toEqual([0xe000, 0xe001]);
        expect(list.errors).toEqual({});
    });
});
