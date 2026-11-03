// Vite takes these at the top level; the other tools pass them through `templateOptions`.
const aliases: Record<string, string> = { classprefix: 'templateoptions', baseselector: 'templateoptions' };

/** One key per option however a tool spells it: `cssDest`, `css_dest` and `--css-dest` are all `cssdest`. */
export const optionKey = (name: string) => {
    const key = name.toLowerCase().replace(/[^a-z0-9]/g, '');
    return aliases[key] ?? key;
};
