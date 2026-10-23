import { beforeEach, expect, it, vi } from 'vite-plus/test';
import { generateWebfonts } from '../src';
import type { Request, Response } from '../src/types';

let worker: EventTarget & { postMessage: ReturnType<typeof vi.fn>; terminate: ReturnType<typeof vi.fn> };
beforeEach(() => {
    worker = Object.assign(new EventTarget(), { postMessage: vi.fn(), terminate: vi.fn() });
    vi.stubGlobal(
        'Worker',
        vi.fn(function () {
            return worker;
        }),
    );
});

function respond(data: Response) {
    worker.dispatchEvent(new MessageEvent('message', { data }));
}

it('uses a custom worker URL and ignores unmatched replies without losing pending calls', async () => {
    const url = new URL('https://example.com/worker.js');
    worker.postMessage.mockImplementation(({ id }: Request) => {
        respond({ id: id + 100, ok: true, value: undefined });
        respond({ id, ok: true, value: undefined });
    });
    using result = await generateWebfonts({}, { workerUrl: url });
    expect(Worker).toHaveBeenCalledWith(url, { type: 'module', name: 'webfont-browser' });
    await expect(result.font('ttf')).resolves.toBeUndefined();
});

it('rejects all pending and future calls when the worker crashes', async () => {
    worker.postMessage.mockImplementationOnce(({ id }: Request) => respond({ id, ok: true, value: undefined }));
    using result = await generateWebfonts({});
    const pending = result.font('ttf');
    worker.dispatchEvent(Object.assign(new Event('error'), { message: 'worker crashed' }));
    await expect(pending).rejects.toThrow('worker crashed');
    await expect(result.generateCss()).rejects.toThrow('worker crashed');
    expect(worker.terminate).toHaveBeenCalledOnce();
});

it('supplies a fallback error when a worker error has no message', async () => {
    const pending = generateWebfonts({});
    worker.dispatchEvent(new Event('error'));
    await expect(pending).rejects.toThrow('Webfont worker failed');
    expect(worker.terminate).toHaveBeenCalledOnce();
});

it('rejects undecodable worker messages', async () => {
    const pending = generateWebfonts({});
    worker.dispatchEvent(new Event('messageerror'));
    await expect(pending).rejects.toThrow('Unable to decode webfont worker response');
    expect(worker.terminate).toHaveBeenCalledOnce();
});

it('rejects synchronous transport failures and permits subsequent requests', async () => {
    worker.postMessage.mockImplementation(({ id }: Request) => respond({ id, ok: true, value: undefined }));
    using result = await generateWebfonts({});
    worker.postMessage.mockImplementationOnce(() => {
        throw new DOMException('Cannot clone', 'DataCloneError');
    });
    await expect(result.font('ttf')).rejects.toThrow('Cannot clone');
    await expect(result.font('ttf')).resolves.toBeUndefined();
    expect(worker.terminate).not.toHaveBeenCalled();
});

it('terminates the worker even when initialization throws a non-Error value', async () => {
    worker.postMessage.mockImplementationOnce(() => {
        throw 'transport unavailable';
    });
    await expect(generateWebfonts({})).rejects.toBe('transport unavailable');
    expect(worker.terminate).toHaveBeenCalledOnce();
});

it('sends cache bytes beside the JSON settings and requests exports', async () => {
    const requests: Request[] = [];
    worker.postMessage.mockImplementation((data: Request) => {
        requests.push(data);
        respond({ id: data.id, ok: true, value: data.method === 'cache' ? new Uint8Array([1]) : undefined });
    });
    const cache = new Uint8Array([7, 8]);
    using result = await generateWebfonts({ fontName: 'cached', cache });
    await expect(result.exportCache()).resolves.toEqual(new Uint8Array([1]));
    expect(requests[0]).toMatchObject({ method: 'generate', cache, options: '{"fontName":"cached"}' });
    expect(requests[1]).toMatchObject({ method: 'cache' });
});
