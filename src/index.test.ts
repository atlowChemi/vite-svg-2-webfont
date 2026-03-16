import { constants } from 'node:fs';
import { access, readFile, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import type { InlineConfig } from 'vite';
import { build, createServer, normalizePath, preview } from 'vite';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { base64ToArrayBuffer, MIME_TYPES } from './utils';

/** Server shape used by tests (avoids relying on vite server types that may not resolve). */
interface TestServer {
    httpServer?: { address(): string | { port: number } | null; close(): void };
    close?(): Promise<void>;
    printUrls?(): void;
}

/** Build output item (Rollup/Rolldown compatible) */
type BuildOutputItem = { type: 'asset' | 'chunk'; fileName?: string; name?: string };
function getBuildOutput(buildResult: unknown): BuildOutputItem[] {
    const first: unknown = Array.isArray(buildResult) ? buildResult[0] : buildResult;
    if (!first || typeof first !== 'object' || !('output' in first)) {
        throw new Error('Build did not return output');
    }
    const output = (first as { output: unknown }).output;
    if (!Array.isArray(output)) {
        throw new Error('Build did not return output');
    }
    return output as BuildOutputItem[];
}

// #region test utils
const root = new URL('./fixtures/', import.meta.url);
const types = ['svg', 'eot', 'woff', 'woff2', 'ttf'];

const normalizeLineBreak = (input: string) => input.replace(/\r\n/g, '\n');
const fileURLToNormalizedPath = (url: URL): string => normalizePath(fileURLToPath(url));

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

const getServerPort = (server: TestServer) => {
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

const fetchFromServer = async (server: TestServer, path: string) => {
    const port = getServerPort(server);
    const url = `http://localhost:${port}${path}`;
    return await fetch(url);
};

const fetchTextContent = async (server: TestServer, path: string) => {
    const res = await fetchFromServer(server, path);
    if (!res.ok || res.status !== 200) {
        return undefined;
    }
    const content = await res.text();
    return normalizeLineBreak(content || '');
};

const fetchBufferContent = async (server: TestServer, path: string) => {
    const res = await fetchFromServer(server, path);
    if (!res.ok || res.status !== 200) {
        return undefined;
    }
    return await res.arrayBuffer();
};

const loadFileContent = async (path: string, encoding: BufferEncoding | 'buffer' = 'utf8'): Promise<string | ArrayBufferLike> => {
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

    let server: TestServer;

    beforeAll(async () => {
        const createdServer = (await createServer(buildConfig)) as { listen(): Promise<TestServer> };
        server = await createdServer.listen();
    });

    afterAll(async () => {
        await server.close?.();
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

    let output: BuildOutputItem[];
    let server: TestServer;
    let cssContent: string | undefined;

    beforeAll(async () => {
        const buildResult = (await build(buildConfig)) as unknown;
        output = getBuildOutput(buildResult);
        server = (await preview(buildConfig)) as TestServer;
        server.printUrls?.();

        const cssAsset = output.find(o => o.type === 'asset' && 'name' in o && o.name === 'index.css');
        if (!cssAsset || cssAsset.type !== 'asset' || !cssAsset.fileName) throw new Error('index.css asset not found');
        cssContent = await fetchTextContent(server, `/${cssAsset.fileName}`);
    });

    afterAll(() => {
        server.httpServer?.close();
    });

    it.concurrent('injects fonts css to page', () => {
        expect(cssContent).toMatch(/^@font-face{font-family:iconfont;/);
    });

    it.concurrent.each(types)('has font of type %s available', async type => {
        const res = await loadFileContent(`fonts/iconfont.${type}`, 'buffer');
        let expected: ArrayBuffer | string | undefined;

        const iconAsset = output.find(
            (o): o is BuildOutputItem & { fileName: string } =>
                o.type === 'asset' && typeof o.fileName === 'string' && o.fileName.startsWith('assets/iconfont-') && o.fileName.endsWith(type),
        );
        if (iconAsset) {
            const iconAssetName = iconAsset.fileName;
            expected = await fetchBufferContent(server, `/${iconAssetName}`);
        } else if (cssContent) {
            // File asset not found in output, check if it's inlined in CSS
            const regex = /url\(data:(?<mime>.+?);base64,(?<data>.*?)\) format\("(?<format>.+?)"\)/g;
            const typeMime = MIME_TYPES[type as keyof typeof MIME_TYPES];
            let m;
            while ((m = regex.exec(cssContent)) !== null) {
                if (m?.groups && 'mime' in m.groups && 'data' in m.groups && m.groups.mime === typeMime) {
                    expected = base64ToArrayBuffer(m.groups.data);
                    break;
                }
            }
        }

        expect(res).not.toEqual(undefined);
        if (expected !== undefined) {
            expect(res).toStrictEqual(expected);
        }
    });
});

describe('build:no-inline', () => {
    const buildConfig = getConfig(ConfigType.NoInline);

    let output: BuildOutputItem[];
    let server: TestServer;
    beforeAll(async () => {
        const buildResult = (await build(buildConfig)) as unknown;
        output = getBuildOutput(buildResult);
        server = (await preview(buildConfig)) as TestServer;
        server.printUrls?.();
    });

    afterAll(() => {
        server.httpServer?.close();
    });

    it.concurrent('injects fonts css to page', async () => {
        const cssAsset = output.find(o => o.type === 'asset' && 'name' in o && o.name === 'index.css');
        if (!cssAsset || cssAsset.type !== 'asset' || !cssAsset.fileName) throw new Error('index.css asset not found');
        const res = await fetchTextContent(server, `/${cssAsset.fileName}`);
        expect(res).toMatch(/^@font-face{font-family:iconfont;/);
    });

    types.forEach(type => {
        it.concurrent.each(types)('has font of type %s available', async () => {
            const iconAsset = output.find(
                (o): o is BuildOutputItem & { fileName: string } =>
                    o.type === 'asset' && typeof o.fileName === 'string' && o.fileName.startsWith('assets/iconfont-') && o.fileName.endsWith(type),
            );
            if (!iconAsset) throw new Error(`iconfont.${type} asset not found`);
            const iconAssetName = iconAsset.fileName;
            const [expected, res] = await Promise.all([loadFileContent(`fonts/iconfont.${type}`, 'buffer'), fetchBufferContent(server, `/${iconAssetName}`)]);
            expect(res).toStrictEqual(expected);
        });
    });
});

describe('build allowWriteFilesInBuild', () => {
    const buildConfig = getConfig(ConfigType.AllowWriteFilesInBuild);

    beforeAll(async () => {
        await (build(buildConfig) as Promise<unknown>);
    });

    afterAll(async () => {
        await rm(new URL('webfont-test/artifacts', root), { recursive: true });
    });

    it.concurrent.each([...types, 'html', 'css'])('has generated font of type %s', async type => {
        const fileName = `webfont-test/artifacts/allowWriteFilesInBuild-test.${type}`;
        const fileNameCasing = types.includes(type) ? fileName : fileName.toLowerCase();
        const filePath = new URL(fileNameCasing, root);

        await expect(access(filePath, constants.F_OK)).resolves.not.toThrow();
    });
});
