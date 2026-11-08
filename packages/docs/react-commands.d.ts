import 'react';

// Invoker commands: a button that opens a dialog with no script. React passes them through; its types don't list them yet.
declare module 'react' {
    interface ButtonHTMLAttributes<T> extends HTMLAttributes<T> {
        command?: 'show-modal' | 'close' | 'request-close' | 'show-popover' | 'hide-popover' | 'toggle-popover' | `--${string}`;
        commandfor?: string;
    }
}
