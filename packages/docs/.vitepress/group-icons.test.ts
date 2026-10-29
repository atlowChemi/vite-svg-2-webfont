import { expect, it } from 'vite-plus/test';
import { groupIconVitePlugin } from 'vitepress-plugin-group-icons';
import { preprocessCSS, resolveConfig } from 'vite-plus';
import { iconCss } from './icon-css';

async function render(labels: string[]) {
    const plugin = groupIconVitePlugin({
        customIcon: {
            alpha: '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h1"/></svg>',
            zeta: '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h1"/></svg>',
            gamma: '<svg xmlns="http://www.w3.org/2000/svg"><path d="M1 1h2"/></svg>',
        },
    });
    const transform = typeof plugin.transform === 'function' ? plugin.transform : plugin.transform!.handler;
    const load = typeof plugin.load === 'function' ? plugin.load : plugin.load!.handler;
    await Promise.all(labels.map(label => Reflect.apply(transform, {}, [`<label data-title="${label}">`, `${label}.md`])));
    const css = await Reflect.apply(load, {}, ['\0virtual:group-icons.css']);
    const config = await resolveConfig({ configFile: false, css: { postcss: iconCss } }, 'build');
    return (await preprocessCSS(css, 'group-icons.css', config)).code;
}

it('generates identical icon CSS regardless of Markdown transform order', async () => {
    const forward = await render(['alpha', 'zeta', 'gamma']);
    expect(forward).toContain("[data-title='alpha']");
    expect(forward).toBe(await render(['gamma', 'zeta', 'alpha']));
});
