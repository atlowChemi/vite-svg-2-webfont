import { expect, it } from 'vite-plus/test';

it('selects distinct designs through production-generated CSS and shared font URLs', async () => {
    const css = await (await fetch('/plugin-family.css')).text();
    const urls = [...css.matchAll(/url\(([^)]+)\)/g)].map(match => match[1]);
    expect(new Set(urls).size).toBe(1);
    const style = document.createElement('style');
    style.textContent = css;
    document.head.append(style);
    const icon = document.createElement('span');
    document.body.append(icon);
    try {
        await Promise.all([300, 700].map(weight => document.fonts.load(`${weight} 100px PluginWeights`, '\ue001')));
        const ink: number[] = [];
        for (const [className, weight] of [
            ['icon icon-add', '300'],
            ['icon icon-add icon--bold', '700'],
        ]) {
            icon.className = className;
            const computed = getComputedStyle(icon, ':before');
            expect(computed.fontWeight).toBe(weight);
            expect(computed.fontSynthesis).toBe('none');
            expect(computed.content).toBe('"\ue001"');
            const canvas = document.createElement('canvas');
            canvas.width = canvas.height = 128;
            const context = canvas.getContext('2d', { willReadFrequently: true })!;
            context.font = `${computed.fontWeight} 100px ${computed.fontFamily}`;
            context.textBaseline = 'top';
            context.fillText('\ue001', 0, 0);
            const pixels = context.getImageData(0, 0, 128, 128).data;
            let alpha = 0;
            for (let index = 3; index < pixels.length; index += 4) alpha += pixels[index];
            ink.push(alpha);
        }
        expect(ink[0]).toBeGreaterThan(0);
        expect(ink[1]).toBeGreaterThan(ink[0] * 2);
    } finally {
        icon.remove();
        style.remove();
    }
});
