import 'virtual:vite-svg-2-webfont.css';
import './style.css';
import { iconBaseSelector, iconClassPrefix, icons } from './webfont/icons';

const baseSelector = iconBaseSelector.replace('.', '');

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
  <div>
    <h1>vite-svg-2-webfont</h1>
    <div id="icons">${icons.map(iconName => {
          const className = `${baseSelector} ${iconClassPrefix}${iconName}`;
          return `<div class="card"><i class="${className}"></i></div>`;
        }).join('\n')
    }</div>
    <a class="read-the-docs" href="https://github.com/ChemiAtlow/vite-svg-2-webfont#readme">
      Read the docs
    </a>
  </div>
`;
