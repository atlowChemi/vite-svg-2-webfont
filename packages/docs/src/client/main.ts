// The browser entry every page links: the fonts, the site's tokens and base styles, the shell's and the Markdown
// output's CSS modules, and the page script.
import '@fontsource-variable/inter/wght.css';
import '@fontsource/zilla-slab/600.css';
import '@fontsource/jetbrains-mono/400.css';
import 'virtual:site-icons.css';
import 'virtual:file-icons.css';
import '../styles/tokens.css';
import '../styles/base.css';
import '../components/Wordmark.module.css';
import '../components/shell/DocShell.module.css';
import '../components/shell/TopBar.module.css';
import '../components/shell/PageStrip.module.css';
import '../components/shell/Rail.module.css';
import '../components/shell/Prose.module.css';
import '../components/shell/PrevNext.module.css';
import '../components/shell/Sheets.module.css';
import '../components/shell/SiteFooter.module.css';
import '../build/markdown.module.css';
import '../build/codeTabs.module.css';
import '../build/highlight.module.css';
import '../build/specimens.module.css';
import '../build/mermaid.module.css';
import '../components/blocks/CacheFileLayout.module.css';
import './client.ts';
