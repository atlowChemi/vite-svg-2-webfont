import { site } from '../../site/site.ts';

/** The page's foot, at its coldest point. */
export function Footer() {
    return (
        <footer className="landing-footer">
            <span className="footer-name">{site.name}</span>
            <a href="/guide/">Docs</a>
            <a href={site.github}>GitHub</a>
            <a href="/rust/changelog">Changelog</a>
            <span className="license mono">MIT license · © Chemi Atlow</span>
        </footer>
    );
}
