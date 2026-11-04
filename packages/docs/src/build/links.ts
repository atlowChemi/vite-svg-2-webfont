import { normalize } from '../site/site.ts';

// VitePress failed the build on a link to a page that doesn't exist; this does the same, and also checks `#hash` links
// against the target page's ids.

export interface LinkedPage {
    route: string;
    html: string;
}

const ids = (html: string) => new Set([...html.matchAll(/\sid="([^"]+)"/g)].map(m => m[1]!));
// Files the site serves besides pages: the public folder and the outputs written next to the pages.
const isFile = (path: string) => /\.\w+$/.test(path) && !path.endsWith('.html');

/** Every internal link that leads nowhere, as `page → link`. */
export function deadLinks(pages: LinkedPage[]): string[] {
    const byRoute = new Map(pages.map(page => [normalize(page.route), ids(page.html)]));
    const dead: string[] = [];
    for (const page of pages) {
        for (const [, href] of page.html.matchAll(/\shref="([^"]+)"/g)) {
            if (/^(?:[a-z]+:|\/\/)/i.test(href!)) continue;
            const [path = '', hash] = href!.replaceAll('&amp;', '&').split('#');
            if (isFile(path)) continue;
            const target = path ? normalize(new URL(path, `https://site${page.route}`).pathname) : normalize(page.route);
            const targetIds = byRoute.get(target);
            if (!targetIds || (hash && !targetIds.has(decodeURIComponent(hash)))) dead.push(`${page.route} → ${href}`);
        }
    }
    return dead;
}
