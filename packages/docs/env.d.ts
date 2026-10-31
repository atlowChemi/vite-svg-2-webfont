declare module 'virtual:group-icons.css';

declare module '*.css' {
    const content: Record<string, string>;
    export default content;
}

declare module '*.vue' {
    import type { DefineComponent } from 'vue';
    const component: DefineComponent<{}, {}, unknown>;
    export default component;
}

declare module 'virtual:svg-authoring-fonts';

declare module 'virtual:docs-examples';

declare module 'virtual:docs-examples-css' {
    const css: Record<string, string>;
    export default css;
}
