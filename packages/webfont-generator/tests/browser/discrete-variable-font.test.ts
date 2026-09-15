import { expect, test } from 'vite-plus/test';

const codepoint = '\ue001';
const ligature = 'ab';
const proofFontUrl = '/discrete-rvrn.woff2';

test('switches unrelated outlines by font weight', async () => {
    const response = await fetch(proofFontUrl);
    expect(response.ok).toBe(true);
    const face = new FontFace('Discrete rvrn proof', await response.arrayBuffer(), {
        style: 'normal',
        weight: '300 700',
    });
    document.fonts.add(face);
    await face.load();

    const samples = [100, 300, 400, 500, 600, 700, 900].map(weight => ({
        direct: render(codepoint, weight),
        ligature: render(ligature, weight),
        weight,
    }));
    const light = samples[0].direct;
    const heavy = samples.at(-1)!.direct;

    for (const sample of samples) {
        expect(sample.direct.opaquePixels).toBeGreaterThan(0);
        expect(sample.ligature).toEqual(sample.direct);
    }
    expect(light.width).toEqual(heavy.width);
    expect(heavy.centroidX - light.centroidX).toBeGreaterThan(40);
    expect(samples[2].direct.centroidX).toBeCloseTo(light.centroidX, 0);
    expect(samples[3].direct.centroidX).toBeCloseTo(heavy.centroidX, 0);
});

test('generated exact-weight faces share a URL and select glyphs and ligatures', async () => {
    const cssResponse = await fetch('/generated-weights.css');
    expect(cssResponse.ok).toBe(true);
    const css = await cssResponse.text();
    expect(css.match(/@font-face/g)).toHaveLength(3);
    expect(css.match(/\/generated-weights\.woff2/g)).toHaveLength(3);
    const style = document.createElement('style');
    style.textContent = css;
    document.head.append(style);
    const element = document.createElement('i');
    document.body.append(element);
    try {
        await Promise.all([300, 400, 700].map(weight => document.fonts.load(`${weight} 100px "GeneratedWeights"`, codepoint)));
        const exact = [300, 400, 700].map(weight => render(codepoint, weight, 'GeneratedWeights'));
        expect(exact[1].centroidX - exact[0].centroidX).toBeGreaterThan(20);
        expect(exact[2].centroidX - exact[1].centroidX).toBeGreaterThan(20);
        for (const [weight, index] of [
            [100, 0],
            [300, 0],
            [350, 0],
            [400, 1],
            [450, 1],
            [500, 1],
            [600, 2],
            [700, 2],
            [900, 2],
        ]) {
            for (const text of [codepoint, ligature]) {
                const sample = render(text, weight, 'GeneratedWeights');
                expect(sample.opaquePixels).toBeGreaterThan(0);
                expect(sample.centroidX).toBeCloseTo(exact[index].centroidX, 0);
            }
        }
        for (const [className, weight] of [
            ['icon-ab', '400'],
            ['icon--light icon-ab', '300'],
            ['icon-ab icon--bold', '700'],
        ]) {
            element.className = className;
            const computed = getComputedStyle(element, ':before');
            expect(computed.fontWeight).toBe(weight);
            expect(computed.fontSynthesis).toBe('none');
            expect(computed.content).toBe(`"${codepoint}"`);
        }
        element.className = 'icon--bold';
        expect(getComputedStyle(element, ':before').content).toBe('none');
    } finally {
        element.remove();
        style.remove();
    }
});

function render(text: string, weight: number, family = 'Discrete rvrn proof') {
    const canvas = document.createElement('canvas');
    canvas.width = 128;
    canvas.height = 128;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) throw new Error('2D canvas is unavailable');

    context.font = `${weight} 100px "${family}"`;
    context.fillStyle = '#000';
    context.textBaseline = 'top';
    const width = context.measureText(text).width;
    context.fillText(text, 0, 0);

    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let opaquePixels = 0;
    let weightedX = 0;
    for (let index = 3; index < pixels.length; index += 4) {
        const alpha = pixels[index];
        if (alpha === 0) continue;
        const x = ((index - 3) / 4) % canvas.width;
        opaquePixels += alpha;
        weightedX += x * alpha;
    }

    return { centroidX: weightedX / opaquePixels, opaquePixels, width };
}
