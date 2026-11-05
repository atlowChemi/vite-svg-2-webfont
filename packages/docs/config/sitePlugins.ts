import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

// The site's own Vite plugins build fonts with vite-svg-2-webfont, which needs its native module. Task discovery loads
// the Vite config before those are built, so the plugins load here, and only `dev` and `build` (which depend on the
// build) require them.

type SitePlugins = typeof import('../src/build/plugins.ts');
let site: SitePlugins | undefined;
let siteError: unknown;
try {
    site = (await import(pathToFileURL(join(import.meta.dirname, '../src/build/plugins.ts')).href)) as SitePlugins;
} catch (error) {
    siteError = error;
}
/** The site plugins' module, or the reason it didn't load, once a build or the dev server actually needs it. */
export const requireSite = (): SitePlugins => {
    if (!site) throw siteError;
    return site;
};

/** The site plugins when they loaded; none during task discovery. */
export const loadedSitePlugins = (): ReturnType<SitePlugins['sitePlugins']> => site?.sitePlugins() ?? [];
