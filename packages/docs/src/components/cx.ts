/** Joins class names, skipping the ones a condition left out. */
export const cx = (...names: (string | false | null | undefined)[]): string => names.filter(Boolean).join(' ');
