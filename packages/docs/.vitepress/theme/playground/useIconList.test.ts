import { ref } from 'vue';
import { describe, expect, it } from 'vite-plus/test';
import { defaultSettings, type PlaygroundSettings } from './options';
import { useIconList } from './useIconList';

const svg = (label: string) => `<svg><title>${label}</title></svg>`;

function setup(overrides: Partial<PlaygroundSettings> = {}) {
    const settings = ref<PlaygroundSettings>({ ...structuredClone(defaultSettings), ...overrides });
    return { settings, ...useIconList(settings) };
}

describe('useIconList', () => {
    it('names icons after their files and numbers them from the start codepoint', () => {
        const { icons, addSources } = setup({ startCodepoint: 0xe001 });
        addSources([
            { fileName: 'add.svg', svg: svg('a') },
            { fileName: 'Arrow Left.svg', svg: svg('b') },
        ]);
        expect(icons.value.map(({ name, codepoint }) => ({ name, codepoint }))).toEqual([
            { name: 'add', codepoint: 0xe001 },
            { name: 'Arrow-Left', codepoint: 0xe002 },
        ]);
    });

    it('replaces the drawing of a same-named icon and keeps its codepoint', () => {
        const { icons, addSources, setCodepoint } = setup();
        addSources([{ fileName: 'add.svg', svg: svg('old') }]);
        setCodepoint(icons.value[0]!, 'e123');
        addSources([{ fileName: 'add.svg', svg: svg('new') }]);
        expect(icons.value).toHaveLength(1);
        expect(icons.value[0]).toMatchObject({ name: 'add', svg: svg('new'), codepoint: 0xe123 });
    });

    it('keeps the same list when there is nothing to add', () => {
        const { icons, addSources } = setup();
        const before = icons.value;
        addSources([]);
        expect(icons.value).toBe(before);
    });

    it('skips codepoints already in use', () => {
        const { icons, addSources, setCodepoint } = setup();
        addSources([{ fileName: 'a.svg', svg: svg('a') }]);
        setCodepoint(icons.value[0]!, 'f102');
        addSources([
            { fileName: 'b.svg', svg: svg('b') },
            { fileName: 'c.svg', svg: svg('c') },
        ]);
        expect(icons.value.map(icon => icon.codepoint)).toEqual([0xf102, 0xf101, 0xf103]);
    });

    it('rejects invalid and duplicate names, and carries color selections across renames', () => {
        const { icons, errors, settings, addSources, rename } = setup({ colorNames: ['a'] });
        addSources([
            { fileName: 'a.svg', svg: svg('a') },
            { fileName: 'b.svg', svg: svg('b') },
        ]);
        const [a, b] = icons.value as [(typeof icons.value)[0], (typeof icons.value)[0]];

        rename(a, 'my icon');
        expect(errors.value[a.id]?.name).toBe('Use letters, numbers, - and _ only. Falling back to a.');
        rename(a, 'b');
        expect(errors.value[a.id]?.name).toBe('Another icon has this name. Falling back to a.');
        expect(icons.value.map(icon => icon.name)).toEqual(['a', 'b']);

        rename(a, 'star');
        expect(errors.value[a.id]?.name).toBeUndefined();
        expect(icons.value.map(icon => icon.name)).toEqual(['star', 'b']);
        expect(settings.value.colorNames).toEqual(['star']);
        expect(icons.value[1]).toBe(b);
    });

    it('rejects unparsable and duplicate codepoints', () => {
        const { icons, errors, addSources, setCodepoint } = setup();
        addSources([
            { fileName: 'a.svg', svg: svg('a') },
            { fileName: 'b.svg', svg: svg('b') },
        ]);
        const b = icons.value[1]!;

        setCodepoint(b, 'zz');
        expect(errors.value[b.id]?.codepoint).toBe('Enter a hexadecimal codepoint, such as F101. Falling back to U+F102.');
        setCodepoint(b, 'F101');
        expect(errors.value[b.id]?.codepoint).toBe('Another icon uses this codepoint. Falling back to U+F102.');
        expect(icons.value[1]!.codepoint).toBe(0xf102);

        setCodepoint(b, 'U+E001');
        expect(errors.value[b.id]?.codepoint).toBeUndefined();
        expect(icons.value[1]!.codepoint).toBe(0xe001);
    });

    it('drops removed icons from the color selection', () => {
        const { icons, settings, addSources, remove } = setup({ colorNames: ['a', 'b'] });
        addSources([
            { fileName: 'a.svg', svg: svg('a') },
            { fileName: 'b.svg', svg: svg('b') },
        ]);
        remove(icons.value[0]!);
        expect(icons.value.map(icon => icon.name)).toEqual(['b']);
        expect(settings.value.colorNames).toEqual(['b']);
    });

    it('renumbers every icon in list order and clears errors', () => {
        const { icons, errors, settings, addSources, setCodepoint, reassign, removeAll } = setup();
        addSources([
            { fileName: 'a.svg', svg: svg('a') },
            { fileName: 'b.svg', svg: svg('b') },
        ]);
        setCodepoint(icons.value[0]!, 'e050');
        setCodepoint(icons.value[1]!, 'nope');
        settings.value = { ...settings.value, startCodepoint: 0xe000 };

        reassign();
        expect(icons.value.map(icon => icon.codepoint)).toEqual([0xe000, 0xe001]);
        expect(errors.value).toEqual({});

        removeAll();
        expect(icons.value).toEqual([]);
    });
});
