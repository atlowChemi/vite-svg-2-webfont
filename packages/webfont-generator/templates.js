import { join } from 'node:path';

const css = join(import.meta.dirname, 'templates', 'css.hbs');
const html = join(import.meta.dirname, 'templates', 'html.hbs');
const scss = join(import.meta.dirname, 'templates', 'scss.hbs');

export { css, html, scss };
