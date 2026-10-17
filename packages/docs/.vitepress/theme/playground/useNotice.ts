import { onScopeDispose, shallowRef } from 'vue';

/** A short-lived message, such as a list of skipped files, that dismisses itself after `duration` ms. */
export function useNotice(duration = 6000) {
    const notice = shallowRef<string>();
    let timer: ReturnType<typeof setTimeout> | undefined;
    function dismiss() {
        clearTimeout(timer);
        notice.value = undefined;
    }
    function notify(message: string) {
        notice.value = message;
        clearTimeout(timer);
        timer = setTimeout(dismiss, duration);
    }
    onScopeDispose(() => clearTimeout(timer));
    return { notice, notify, dismiss };
}
