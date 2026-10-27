import { expect, it } from 'vite-plus/test';
import { groupIconVitePlugin } from 'vitepress-plugin-group-icons';

async function render(labels: string[]) {
    const plugin = groupIconVitePlugin({
        customIcon: {
            alpha: '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h1"/></svg>',
            beta: '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h1"/></svg>',
            gamma: '<svg xmlns="http://www.w3.org/2000/svg"><path d="M1 1h2"/></svg>',
        },
    });
    const transform = typeof plugin.transform === 'function' ? plugin.transform : plugin.transform!.handler;
    const load = typeof plugin.load === 'function' ? plugin.load : plugin.load!.handler;
    await Promise.all(labels.map(label => Reflect.apply(transform, {}, [`<label data-title="${label}">`, `${label}.md`])));
    return Reflect.apply(load, {}, ['\0virtual:group-icons.css']);
}

it('generates identical icon CSS regardless of Markdown transform order', async () => {
    const forward = await render(['alpha', 'beta', 'gamma']);
    expect(forward).toContain("[data-title='alpha']");
    expect(forward).toBe(await render(['gamma', 'beta', 'alpha']));
});
