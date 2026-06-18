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
