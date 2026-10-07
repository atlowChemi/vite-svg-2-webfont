/// <reference types="vite-plus/client" />
import { expect, it } from 'vite-plus/test';

const webkit = /AppleWebKit/.test(navigator.userAgent) && !/Chrome/.test(navigator.userAgent);
const platform = import.meta.env.VITE_COLOR_PROOF_PLATFORM;
// Approved compatibility contract: macOS WebKit 26.6 uses monochrome fallback;
// Linux WebKit must pass full color assertions. Its UA also claims macOS.
// VITE_COLOR_PROOF_STRICT=1 is an optional future-support diagnostic, not the
// release gate. Color remains mandatory in Chromium and Firefox.
const recordFallback = webkit && platform === 'darwin' && import.meta.env.VITE_COLOR_PROOF_STRICT !== '1';

// These fonts use the production SVG-to-COLR pipeline via the Rust color_proof task.
for (const format of ['ttf', 'woff', 'woff2']) {
    it(`${recordFallback ? 'WebKit preserves the approved monochrome fallback' : 'COLR v1 follows rvrn and conditioned liga'} in ${format}`, async () => {
        console.info(`COLR proof host: ${platform}; browser: ${navigator.userAgent}`);
        const response = await fetch(`/color-rvrn.${format}`);
        expect(response.ok).toBe(true);
        const family = `ColorProof-${format}`;
        const face = new FontFace(family, await response.arrayBuffer(), { weight: '300 700' });
        document.fonts.add(face);
        await face.load();
        try {
            for (const weight of [100, 300, 400, 499, 500, 501, 600, 700, 900]) {
                const bold = weight >= 500;
                for (const foreground of ['#00ff00', '#ff00ff']) {
                    const direct = render('\ue001', weight, foreground, family);
                    const ligature = render('ab', weight, foreground, family);
                    expect(ligature).toEqual(direct);
                    expect(direct.width).toBeCloseTo(100, 1);
                    const fixed = bold ? [0, 0, 255] : [255, 0, 0];
                    const host = foreground === '#00ff00' ? [0, 255, 0] : [255, 0, 255];
                    const opaque = pixels(direct.data, 254, 255);
                    const translucent = pixels(direct.data, 62, 65);
                    expect(opaque.length).toBeGreaterThan(1000);
                    if (recordFallback) {
                        assertMonochromeFallback(opaque, translucent, host, bold);
                        continue;
                    }
                    expect(translucent.length).toBeGreaterThan(1000);
                    for (const pixel of opaque) expect(pixel.rgb).toEqual(fixed);
                    for (const pixel of translucent) expect(pixel.rgb).toEqual(host);
                    const fixedX = opaque.reduce((sum, pixel) => sum + pixel.x, 0) / opaque.length;
                    const foregroundX = translucent.reduce((sum, pixel) => sum + pixel.x, 0) / translucent.length;
                    expect(fixedX).toBeCloseTo(bold ? 79.5 : 19.5, 0);
                    expect(foregroundX).toBeCloseTo(bold ? 19.5 : 69.5, 0);
                }
            }
        } finally {
            document.fonts.delete(face);
        }
    });
}

function render(text: string, weight: number, foreground: string, family: string) {
    const canvas = document.createElement('canvas');
    canvas.width = 128;
    canvas.height = 128;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) throw new Error('2D canvas is unavailable');
    context.font = `${weight} 100px "${family}"`;
    context.textBaseline = 'top';
    context.fillStyle = foreground;
    const width = context.measureText(text).width;
    context.fillText(text, 0, 0);
    return { width, data: Array.from(context.getImageData(0, 0, 128, 128).data) };
}

function assertMonochromeFallback(opaque: ReturnType<typeof pixels>, translucent: ReturnType<typeof pixels>, host: number[], bold: boolean) {
    for (const pixel of opaque) expect(pixel.rgb).toEqual(host);
    expect(translucent.length).toBeLessThan(10);
    const centroid = opaque.reduce((sum, pixel) => sum + pixel.x, 0) / opaque.length;
    // The fallback contains both source paths, with no independent layer alpha.
    expect(centroid).toBeCloseTo(bold ? 49.5 : 44.5, 0);
}

function pixels(data: number[], minAlpha: number, maxAlpha: number) {
    const result = [];
    for (let index = 0; index < data.length; index += 4) {
        if (data[index + 3] >= minAlpha && data[index + 3] <= maxAlpha) {
            result.push({ rgb: data.slice(index, index + 3), x: (index / 4) % 128 });
        }
    }
    return result;
}
