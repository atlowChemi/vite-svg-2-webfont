import type { Options, SearchOptions } from 'minisearch';

// Shared by the build, which writes the index, and the search dialog, which loads it: both sides must agree.
export interface SearchDoc {
    id: string;
    title: string;
    titles: string[];
    text: string;
}

export const indexOptions: Options<SearchDoc> = {
    fields: ['title', 'titles', 'text'],
    // The text is stored too, for each result's excerpt.
    storeFields: ['title', 'titles', 'text'],
    stringifyField: (value, field) => (field === 'titles' ? (value as string[]).join(' ') : String(value)),
};

export const searchOptions: SearchOptions = { boost: { title: 4, text: 2, titles: 1 }, fuzzy: 0.2, prefix: true };
