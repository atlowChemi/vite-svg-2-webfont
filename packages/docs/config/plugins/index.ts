// The site's own Vite plugins. They build fonts with vite-svg-2-webfont, so `../sitePlugins.ts` loads this module only
// once the plugin and its native module are built; task discovery runs before that, with no plugins at all.
import type { Plugin } from 'vite';
import { examplePlugins } from './examples.ts';
import { fileIcons } from './fileIcons.ts';
import { siteIconFont } from './siteIconFont.ts';
import { svgAuthoringFontPlugins } from './svgAuthoringFonts.ts';

export { setFileIconLabels } from './fileIcons.ts';

export const sitePlugins = (): Plugin[] => [fileIcons(), siteIconFont(), ...svgAuthoringFontPlugins(), ...examplePlugins()];
