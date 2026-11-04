import { useEffect, useRef, useState } from 'react';

/** A short-lived message, such as a list of skipped files, that dismisses itself after `duration` ms. */
export function useNotice(duration = 6000) {
    const [notice, setNotice] = useState<string>();
    const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
    useEffect(() => () => clearTimeout(timer.current), []);
    function dismiss() {
        clearTimeout(timer.current);
        setNotice(undefined);
    }
    function notify(message: string) {
        setNotice(message);
        clearTimeout(timer.current);
        timer.current = setTimeout(dismiss, duration);
    }
    return { notice, notify, dismiss };
}
