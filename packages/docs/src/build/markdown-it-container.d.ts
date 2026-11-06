// markdown-it 15 ships its own types; the container plugin's DefinitelyTyped package targets the older ones.
declare module 'markdown-it-container' {
    import type { MarkdownIt, Token } from 'markdown-it';

    interface ContainerOptions {
        render?(tokens: Token[], index: number): string;
        validate?(params: string): boolean;
        marker?: string;
    }
    const container: (md: MarkdownIt, name: string, options?: ContainerOptions) => void;
    export default container;
}
