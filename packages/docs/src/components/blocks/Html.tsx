import type { ElementType } from 'react';

/** Markup rendered elsewhere (a Markdown slot, highlighted code, an SVG), placed inside a wrapper element. */
export function Html({ html, as: Tag = 'div', className, title }: { html: string; as?: ElementType; className?: string; title?: string }) {
    return <Tag className={className} title={title} dangerouslySetInnerHTML={{ __html: html }} />;
}
