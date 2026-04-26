import type { GeneratedWebfont } from './generatedWebfont';

/**
 * The public API exposed by the plugin, accessible via the rollup / rolldown plugin system.
 */
export interface PublicApi {
    /** Returns an array of generated webfonts. */
    getGeneratedWebfonts(): GeneratedWebfont[];
}
