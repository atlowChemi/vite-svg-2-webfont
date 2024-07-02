import { constants } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { readFile, access, rmdir } from 'node:fs/promises';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import { build, createServer, preview, normalizePath } from 'vite';
import type { RollupOutput } from 'rollup';
import type { PreviewServer, ViteDevServer, InlineConfig } from 'vite';
import { base64ToArrayBuffer } from './utils';

// #region test utils
const root = new URL('./fixtures/', import.meta.url);
const types = ['svg', 'eot', 'woff', 'woff2', 'ttf'];

const normalizeLineBreak = (input: string) => input.replace(/\r\n/g, '\n');
const fileURLToNormalizedPath = (url: URL) => normalizePath(fileURLToPath(url));

const enum ConfigType {
    Basic = './vite.basic.config.ts',
    NoInline = './vite.no-inline.config.ts',
    AllowWriteFilesInBuild = './vite.allowWriteFilesInBuild.config.ts',
}
const getConfig = (configType: ConfigType): InlineConfig => ({
    logLevel: 'silent',
    root: fileURLToNormalizedPath(root),
    configFile: fileURLToNormalizedPath(new URL(configType, root)),
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
    const buildConfig = getConfig(ConfigType.Basic);

    let server: ViteDevServer;

    beforeAll(async () => {
        const createdServer = await createServer(buildConfig);
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
    const buildConfig = getConfig(ConfigType.Basic);

    let output: RollupOutput['output'];
    let server: PreviewServer;
    let cssContent: string | undefined;

    const typeToMimeMap: Record<string, string> = {
        svg: 'image/svg+xml',
        eot: 'application/vnd.ms-fontobject',
        woff: 'font/woff',
        woff2: 'font/woff2',
        ttf: 'font/ttf',
    };

    beforeAll(async () => {
        output = ((await build(buildConfig)) as RollupOutput).output;
        server = await preview(buildConfig);
        server.printUrls();

        const cssFileName = output.find(({ type, name }) => type === 'asset' && name === 'index.css')!.fileName;
        cssContent = await fetchTextContent(server, `/${cssFileName}`);
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('injects fonts css to page', async () => {
        expect(cssContent).toMatch(/^@font-face{font-family:iconfont;/);
    });

    types.forEach(async type => {
        it.concurrent(`has font of type ${type} available`, async () => {
            const res = await loadFileContent(`fonts/iconfont.${type}`, 'buffer');
            let expected: ArrayBuffer | string | undefined;

            const iconAsset = output.find(({ fileName }) => fileName.startsWith('assets/iconfont-') && fileName.endsWith(type));
            if (iconAsset) {
                const iconAssetName = iconAsset.fileName;
                expected = await fetchBufferContent(server, `/${iconAssetName}`);
            } else if (cssContent) {
                // File asset not found in output, check if it's inlined in CSS

                const regex = /url\(data:(?<mime>.+?);base64,(?<data>.*?)\) format\("(?<format>.+?)"\)/g;

                let m;
                while ((m = regex.exec(cssContent)) !== null) {
                    if (m?.groups && 'mime' in m.groups && 'data' in m.groups) {
                        const typeMime = typeToMimeMap[type];
                        if (m.groups.mime === typeMime) {
                            expected = base64ToArrayBuffer(m.groups.data);
                        }
                    }
                }
            }

            expect(res).not.toEqual(undefined);
            expect(res).toStrictEqual(expected);
        });
    });
});

describe('build:no-inline', () => {
    const buildConfig = getConfig(ConfigType.NoInline);

    let output: RollupOutput['output'];
    let server: PreviewServer;
    beforeAll(async () => {
        output = ((await build(buildConfig)) as RollupOutput).output;
        server = await preview(buildConfig);
        server.printUrls();
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('injects fonts css to page', async () => {
        const cssFileName = output.find(({ type, name }) => type === 'asset' && name === 'index.css')!.fileName;
        const res = await fetchTextContent(server, `/${cssFileName}`);
        expect(res).toMatch(/^@font-face{font-family:iconfont;/);
    });

    types.forEach(type => {
        it.concurrent.each(types)('has font of type %s available', async () => {
            const iconAssetName = output.find(({ fileName }) => fileName.startsWith('assets/iconfont-') && fileName.endsWith(type))!.fileName;
            const [expected, res] = await Promise.all([loadFileContent(`fonts/iconfont.${type}`, 'buffer'), fetchBufferContent(server, `/${iconAssetName}`)]);
            expect(res).toStrictEqual(expected);
        });
    });
});

describe('build allowWriteFilesInBuild', () => {
    const buildConfig = getConfig(ConfigType.AllowWriteFilesInBuild);

    beforeAll(async () => {
        await build(buildConfig);
    });

    afterAll(async () => {
        await rmdir(new URL('webfont-test/artifacts', root), { recursive: true });
    });

    it.concurrent.each([...types, 'html', 'css'])('has generated font of type %s', async type => {
        const filePath = new URL(`webfont-test/artifacts/allowWriteFilesInBuild-test.${type}`.toLowerCase(), root);

        await expect(access(filePath, constants.F_OK)).resolves.not.toThrow();
    });
});
