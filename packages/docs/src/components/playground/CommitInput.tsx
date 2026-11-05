import { useEffect, useEffectEvent, useRef, useState, type ComponentProps, type FC } from 'react';

interface CommitInputProps extends Omit<ComponentProps<'input'>, 'value' | 'defaultValue' | 'onChange'> {
    value: string;
    /** Called with the typed text once it is committed (Enter, or leaving a changed field), not on every keystroke. */
    onCommit: (text: string) => void;
}

/**
 * A text field that edits a draft and commits it on the DOM's `change` event, which React's `onChange` does not
 * expose. A rejected draft stays in the field; it resets whenever `value` changes.
 */
export const CommitInput: FC<CommitInputProps> = ({ value, onCommit, ...props }) => {
    const [draft, setDraft] = useState(value);
    const [shown, setShown] = useState(value);
    if (value !== shown) {
        setShown(value);
        setDraft(value);
    }
    const input = useRef<HTMLInputElement>(null);
    const commit = useEffectEvent(() => onCommit(input.current!.value));
    useEffect(() => {
        const field = input.current!;
        const listener = () => commit();
        field.addEventListener('change', listener);
        return () => field.removeEventListener('change', listener);
    }, []);
    return <input {...props} ref={input} value={draft} onChange={event => setDraft(event.target.value)} />;
};
