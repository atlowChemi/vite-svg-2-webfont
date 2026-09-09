import { join } from 'node:path';
import type { Plugin } from 'vite-plus';

export function generatedWeightFixture(): Plugin {
    return {
        name: 'generated-weight-fixture',
        async configureServer(server) {
            // Load the native binding only when this browser project's server starts.
            const { generateWebfonts } = await import('../../index.js');
            const result = await generateWebfonts({
                dest: 'artifacts',
                fontName: 'GeneratedWeights',
                types: ['woff2'],
                writeFiles: false,
                normalize: false,
                codepoints: { ab: 0xe001 },
                templateOptions: { baseSelector: '[class^="icon-"]' },
                variants: [
                    { name: 'light', weight: 300, files: [join(import.meta.dirname, 'fixtures/weights/light/ab.svg')] },
                    { name: 'regular', weight: 400, default: true, files: [join(import.meta.dirname, 'fixtures/weights/regular/ab.svg')] },
                    { name: 'bold', weight: 700, files: [join(import.meta.dirname, 'fixtures/weights/bold/ab.svg')] },
                ],
            });
            const css = result.generateCss({ woff2: '/generated-weights.woff2' });
            server.middlewares.use((request, response, next) => {
                if (request.url === '/generated-weights.css') {
                    response.setHeader('Content-Type', 'text/css');
                    response.end(css);
                } else if (request.url === '/generated-weights.woff2') {
                    response.setHeader('Content-Type', 'font/woff2');
                    response.end(result.woff2);
                } else {
                    next();
                }
            });
        },
    };
}
