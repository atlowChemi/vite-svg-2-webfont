// The page's theme: the one the reader picked, as a `light` or `dark` class on the root, or else the system's. CSS reads
// the same through `color-scheme` (`base.css`); this is for the scripts that need it.
const root = document.documentElement;

export const systemDark = matchMedia('(prefers-color-scheme: dark)');

export const isDark = () => root.classList.contains('dark') || (!root.classList.contains('light') && systemDark.matches);
