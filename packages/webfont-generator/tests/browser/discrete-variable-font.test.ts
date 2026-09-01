import { expect, test } from 'vite-plus/test';

const codepoint = '\ue001';
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

    const light = render(300);
    const heavy = render(700);

    expect(light.opaquePixels).toBeGreaterThan(0);
    expect(heavy.opaquePixels).toBeGreaterThan(0);
    expect(light.width).toBeCloseTo(heavy.width, 2);
    expect(heavy.centroidX - light.centroidX).toBeGreaterThan(40);
});

function render(weight: number) {
    const canvas = document.createElement('canvas');
    canvas.width = 128;
    canvas.height = 128;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) throw new Error('2D canvas is unavailable');

    context.font = `${weight} 100px "Discrete rvrn proof"`;
    context.fillStyle = '#000';
    context.textBaseline = 'top';
    const width = context.measureText(codepoint).width;
    context.fillText(codepoint, 0, 0);

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
