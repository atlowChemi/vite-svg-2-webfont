import { useEffect, useRef, useState } from 'react';

/** Copy text and remember which `label` was copied for `resetAfter` ms, for "Copied!" feedback. */
export function useClipboard(resetAfter = 1500) {
    const [copied, setCopied] = useState<string>();
    const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
    useEffect(() => () => clearTimeout(timer.current), []);
    async function copy(label: string, text: string) {
        try {
            await navigator.clipboard.writeText(text);
            setCopied(label);
            clearTimeout(timer.current);
            timer.current = setTimeout(() => setCopied(undefined), resetAfter);
        } catch {
            setCopied(undefined);
        }
    }
    return { copied, copy };
}
