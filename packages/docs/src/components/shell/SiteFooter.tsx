import { repo } from '../../site/site.ts';

const changelogs = [
    { text: 'Vite Plugin', link: '/vite/changelog' },
    { text: 'Node.js', link: '/node/changelog' },
    { text: 'Browser', link: '/browser/changelog' },
    { text: 'Rust Engine & CLI', link: '/rust/changelog' },
];
const registries = [
    { text: 'crates.io', link: 'https://crates.io/crates/webfont-generator' },
    { text: 'npm', link: 'https://www.npmjs.com/package/vite-svg-2-webfont' },
    { text: 'GitHub', link: repo },
];

/** The foot of the full-width pages: changelogs, where the packages live, and the license. */
export function SiteFooter() {
    return (
        <footer className="doc-footer">
            <nav aria-label="Changelogs">
                <span className="plate">Changelogs</span>
                {changelogs.map(c => (
                    <a key={c.link} href={c.link}>
                        {c.text}
                    </a>
                ))}
            </nav>
            <nav aria-label="Packages">
                {registries.map(r => (
                    <a key={r.link} href={r.link} target="_blank" rel="noreferrer">
                        {r.text}
                    </a>
                ))}
            </nav>
            <p className="license">Released under the MIT License. Copyright © Chemi Atlow</p>
        </footer>
    );
}
