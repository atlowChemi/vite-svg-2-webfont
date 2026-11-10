import type { Browser } from 'playwright';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DiagramBrowser } from './diagramBrowser.ts';

const { launch } = vi.hoisted(() => ({ launch: vi.fn<() => Promise<Browser>>() }));
vi.mock('playwright', () => ({ chromium: { launch } }));

/** A stand-in Chromium whose tabs only remember what they were asked to do. */
function fakeBrowser() {
    const browser = {
        visited: [] as string[],
        close: vi.fn(async () => {}),
        newPage: vi.fn(async () => ({
            goto: async (url: string) => void browser.visited.push(url),
            close: async () => {},
        })),
    };
    return browser;
}

let browser: ReturnType<typeof fakeBrowser>;
beforeEach(() => {
    launch.mockReset();
    browser = fakeBrowser();
    launch.mockImplementation(async () => browser as unknown as Browser);
});

describe('DiagramBrowser', () => {
    it('launches one Chromium for tabs asked for at the same time', async () => {
        const diagrams = new DiagramBrowser();
        await using _held = diagrams.use();
        await Promise.all([diagrams.tab(), diagrams.tab(), diagrams.tab()]);
        expect(launch).toHaveBeenCalledOnce();
        expect(browser.newPage).toHaveBeenCalledTimes(3);
    });

    it('opens each tab on the page that loads Mermaid', async () => {
        const diagrams = new DiagramBrowser();
        await using _held = diagrams.use();
        await diagrams.tab();
        expect(browser.visited).toEqual([expect.stringMatching(/^file:\/\/.*\/\.cache\/mermaid\.html$/)]);
    });

    it('stays open while anyone holds it, and closes when the last one lets go', async () => {
        const diagrams = new DiagramBrowser();
        const first = diagrams.use();
        const second = diagrams.use();
        await diagrams.tab();
        await first[Symbol.asyncDispose]();
        expect(browser.close).not.toHaveBeenCalled();
        await second[Symbol.asyncDispose]();
        expect(browser.close).toHaveBeenCalledOnce();
    });

    it('lets go of a handle only once, however often it is disposed', async () => {
        const diagrams = new DiagramBrowser();
        const first = diagrams.use();
        await using _second = diagrams.use();
        await diagrams.tab();
        await first[Symbol.asyncDispose]();
        await first[Symbol.asyncDispose]();
        expect(browser.close).not.toHaveBeenCalled();
    });

    it('closes nothing when no tab was ever asked for', async () => {
        const diagrams = new DiagramBrowser();
        await diagrams.use()[Symbol.asyncDispose]();
        expect(launch).not.toHaveBeenCalled();
    });

    it('launches a fresh Chromium once held again after closing', async () => {
        const diagrams = new DiagramBrowser();
        {
            await using _held = diagrams.use();
            await diagrams.tab();
        }
        const closed = browser;
        browser = fakeBrowser();
        await using _held = diagrams.use();
        await diagrams.tab();
        expect(launch).toHaveBeenCalledTimes(2);
        expect(closed.close).toHaveBeenCalledOnce();
        expect(browser.close).not.toHaveBeenCalled();
    });

    it('tries a failed launch again on the next tab', async () => {
        const diagrams = new DiagramBrowser();
        await using _held = diagrams.use();
        launch.mockRejectedValueOnce(new Error('no Chromium'));
        await expect(diagrams.tab()).rejects.toThrow('no Chromium');
        await diagrams.tab();
        expect(launch).toHaveBeenCalledTimes(2);
    });

    it('lets go without throwing while a launch that will fail is still under way', async () => {
        const diagrams = new DiagramBrowser();
        const { promise, reject } = Promise.withResolvers<Browser>();
        launch.mockReturnValueOnce(promise);
        const held = diagrams.use();
        const tab = diagrams.tab();
        await vi.waitFor(() => expect(launch).toHaveBeenCalled());
        const released = held[Symbol.asyncDispose]();
        reject(new Error('no Chromium'));
        await Promise.all([expect(released).resolves.toBeUndefined(), expect(tab).rejects.toThrow('no Chromium')]);
    });
});
