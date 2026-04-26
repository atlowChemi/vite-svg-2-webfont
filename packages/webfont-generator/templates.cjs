const path = require('node:path');

const css = path.join(__dirname, 'templates', 'css.hbs');
const html = path.join(__dirname, 'templates', 'html.hbs');
const scss = path.join(__dirname, 'templates', 'scss.hbs');

module.exports = { css, html, scss };
