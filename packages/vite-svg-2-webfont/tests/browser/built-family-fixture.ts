import { join } from 'node:path';
import { build, type Plugin } from 'vite-plus';
import { viteSvgToWebfont } from '../../src';

/** Exercise the real production plugin and serve its emitted bytes to the browser test. */
export function builtFamilyFixture(): Plugin {
    return {
        name: 'built-plugin-family-fixture',
        async configureServer(server) {
            const root = join(import.meta.dirname, 'fixtures');
            const bundle = await build({
                root,
                configFile: false,
                logLevel: 'silent',
                plugins: [
                    viteSvgToWebfont({
                        context: root,
                        fontName: 'PluginWeights',
                        types: ['woff2'],
                        normalize: false,
                        codepoints: { add: 0xe001 },
                        variants: [
                            { name: 'light', context: 'light', weight: 300, default: true },
                            { name: 'bold', context: 'bold', weight: 700 },
                        ],
                    }),
                ],
                build: { write: false, assetsInlineLimit: 0 },
            });
            if (Array.isArray(bundle) || !('output' in bundle)) throw new Error('Expected one plugin fixture bundle');
            const assets = bundle.output.filter(chunk => chunk.type === 'asset');
            const css = assets.find(asset => asset.fileName.endsWith('.css'))!;
            server.middlewares.use((request, response, next) => {
                const asset = request.url === '/plugin-family.css' ? css : assets.find(candidate => `/${candidate.fileName}` === request.url);
                if (!asset) return next();
                response.setHeader('Content-Type', asset.fileName.endsWith('.css') ? 'text/css' : 'font/woff2');
                response.end(asset.source);
            });
        },
    };
}
