import type { Plugin } from 'vite';
import type { GeneratedWebfont } from './generatedWebfont';

/**
 * Compatibility type for Vite plugins.
 *
 * Generic API support was added in vite 5+, this type is needed to also support older versions.
 */
export interface CompatiblePlugin<T> extends Plugin {
    api?: T;
}
export interface PublicApi {
    getGeneratedWebfonts(): GeneratedWebfont[];
}
