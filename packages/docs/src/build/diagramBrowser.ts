import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium, type Browser, type Page } from 'playwright';

const mermaidScript = join(dirname(fileURLToPath(import.meta.resolve('mermaid'))), 'mermaid.min.js');
// A tab loads Mermaid from this page, so Chromium reads the 3 MB script from disk itself: sent through Playwright with
// `addScriptTag`, it took about three times as long, and tabs sharing a browser waited on each other. In the docs' git-ignored
// `.cache`, two folders above this module from `src/build` and from `.cache/server`.
const mermaidPage = join(import.meta.dirname, '..', '..', '.cache', 'mermaid.html');

/**
 * The Chromium the diagrams render in: launched by the first diagram that isn't cached, and shared by every page after
 * it. Each page renders in a tab of its own, since Mermaid keeps global state. Whoever renders holds it with `use()`,
 * and the last holder to let go closes it, so no caller closes it under another; held again, it launches again.
 */
export class DiagramBrowser {
    #browserPromise: Promise<Browser> | undefined;
    #users = 0;

    /** Keeps the browser open until the returned handle is disposed (`await using`). */
    use(): AsyncDisposable {
        this.#users++;
        let released = false;
        return {
            [Symbol.asyncDispose]: async () => {
                if (released) return;
                released = true;
                if (--this.#users === 0) await this.#close();
            },
        };
    }

    /** A new tab with Mermaid loaded, for a caller that holds the browser with `use()`. Dispose it when the page's diagrams are done. */
    async tab(): Promise<Page> {
        this.#browserPromise ??= this.#launch();
        const browser = await this.#browserPromise;
        const page = await browser.newPage();
        await page.goto(pathToFileURL(mermaidPage).href);
        return page;
    }

    async #launch(): Promise<Browser> {
        try {
            await mkdir(dirname(mermaidPage), { recursive: true });
            const [browser] = await Promise.all([chromium.launch(), writeFile(mermaidPage, `<!doctype html><script src="${pathToFileURL(mermaidScript).href}"></script>`)]);
            return browser;
        } catch (error) {
            // A failed launch is tried again by the next diagram.
            this.#browserPromise = undefined;
            throw error;
        }
    }

    async #close(): Promise<void> {
        // Cleared first, so a holder arriving while it closes launches a fresh one.
        const browser = this.#browserPromise;
        this.#browserPromise = undefined;
        await (await browser?.catch(() => undefined))?.close();
    }
}

export const diagramBrowser = new DiagramBrowser();
