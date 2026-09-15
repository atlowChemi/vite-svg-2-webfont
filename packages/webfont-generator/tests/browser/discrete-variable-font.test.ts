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

function render(text: string, weight: number) {
    const canvas = document.createElement('canvas');
    canvas.width = 128;
    canvas.height = 128;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) throw new Error('2D canvas is unavailable');

    context.font = `${weight} 100px "Discrete rvrn proof"`;
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
