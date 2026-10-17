import { onScopeDispose, shallowRef } from 'vue';

/** Copy text and remember which `label` was copied for `resetAfter` ms, for "Copied!" feedback. */
export function useClipboard(resetAfter = 1500) {
    const copied = shallowRef<string>();
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function copy(label: string, text: string) {
        try {
            await navigator.clipboard.writeText(text);
            copied.value = label;
            clearTimeout(timer);
            timer = setTimeout(() => (copied.value = undefined), resetAfter);
        } catch {
            copied.value = undefined;
        }
    }
    onScopeDispose(() => clearTimeout(timer));
    return { copied, copy };
}
