import { fileURLToPath } from 'url';
import { readFile } from 'fs/promises';
import nodeFetch from 'node-fetch';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import { build, createServer, preview, normalizePath } from 'vite';
import type { RollupOutput } from 'rollup';
import type { PreviewServer, ViteDevServer, InlineConfig } from 'vite';

// Currently @types/node doesn't include the fetch typing yet.
declare global {
    let fetch: typeof import('node-fetch').default;
}
fetch ||= nodeFetch;

// #region test utils
const root = new URL('./fixtures/', import.meta.url);
const types = ['svg', 'eot', 'woff', 'woff2', 'ttf'];

const normalizeLineBreak = (input: string) => input.replace(/\r\n/g, '\n');
const fileURLToNormalizedPath = (url: URL) => normalizePath(fileURLToPath(url));

const getConfig = (): InlineConfig => ({
    logLevel: 'silent',
    root: fileURLToNormalizedPath(root),
    configFile: fileURLToNormalizedPath(new URL('./vite.basic.config.ts', root)),
});

const getServerPort = (server: ViteDevServer | PreviewServer) => {
    const address = server.httpServer?.address();
    if (!address) {
        throw new Error('Address not found');
    }
    if (typeof address === 'string') {
        const [, port] = address.split(':');
        return parseInt(port || '80');
    }
    return address.port;
};

const fetchFromServer = async (server: ViteDevServer | PreviewServer, path: string) => {
    const port = getServerPort(server);
    const url = `http://localhost:${port}${path}`;
    return await fetch(url);
};

const fetchTextContent = async (server: ViteDevServer | PreviewServer, path: string) => {
    const res = await fetchFromServer(server, path);
    if (!res.ok || res.status !== 200) {
        return undefined;
    }
    const content = await res.text();
    return normalizeLineBreak(content || '');
};

const fetchBufferContent = async (server: ViteDevServer | PreviewServer, path: string) => {
    const res = await fetchFromServer(server, path);
    if (!res.ok || res.status !== 200) {
        return undefined;
    }
    return await res.arrayBuffer();
};

const loadFileContent = async (path: string, encoding: BufferEncoding | 'buffer' = 'utf8'): Promise<string | ArrayBuffer> => {
    const absolutePath = new URL(path, root);
    const content = await readFile(absolutePath, encoding === 'buffer' ? null : encoding);

    if (typeof content !== 'string') {
        return content.buffer;
    }
    return normalizeLineBreak(content);
};
// #endregion

describe('serve - handles virtual import and has all font types available', () => {
    let server: ViteDevServer;

    beforeAll(async () => {
        const createdServer = await createServer(getConfig());
        server = await createdServer.listen();
    });

    afterAll(async () => {
        await server.close();
    });

    it.concurrent('handles virtual import', async () => {
        const res = await fetchTextContent(server, `/main.ts`);
        expect(res).toMatch(/^import "\/@id\/__x00__virtual:vite-svg-2-webfont\.css";/);
    });

    types.forEach(type => {
        it.concurrent(`has font of type ${type} available`, async () => {
            const [expected, res] = await Promise.all([loadFileContent(`fonts/iconfont.${type}`, 'buffer'), fetchBufferContent(server, `/iconfont.${type}`)]);
            expect(res).toStrictEqual(expected);
        });
    });
});

describe('build', () => {
    let output: RollupOutput['output'];
    let server: PreviewServer;
    beforeAll(async () => {
        output = ((await build(getConfig())) as RollupOutput).output;
        server = await preview(getConfig());
        server.printUrls();
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('injects fonts css to page', async () => {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const cssFileName = output.find(({ type, name }) => type === 'asset' && name === 'index.css')!.fileName;
        const res = await fetchTextContent(server, `/${cssFileName}`);
        expect(res).toMatch(/^@font-face{font-family:iconfont;/);
    });

    types.forEach(type => {
        it.concurrent(`has font of type ${type} available`, async () => {
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            const iconAssetName = output.find(({ fileName }) => fileName.startsWith('assets/iconfont-') && fileName.endsWith(type))!.fileName;
            const [expected, res] = await Promise.all([loadFileContent(`fonts/iconfont.${type}`, 'buffer'), fetchBufferContent(server, `/${iconAssetName}`)]);
            expect(res).toStrictEqual(expected);
        });
    });
});
