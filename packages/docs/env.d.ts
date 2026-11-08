declare module 'virtual:file-icons.css';

declare module 'virtual:site-icons.css';

declare module '*.css' {
    const content: Record<string, string>;
    export default content;
}

declare module 'virtual:svg-authoring-fonts';

declare module 'virtual:docs-examples';

declare module 'virtual:docs-examples-css' {
    const css: Record<string, string>;
    export default css;
}
