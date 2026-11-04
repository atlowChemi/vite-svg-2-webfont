import { Fragment, useId } from 'react';
import { Html } from './Html.tsx';

/**
 * A code group whose tabs can hold any content, not only code blocks. `tabs` maps each slot name to its tab label, in
 * display order; the first tab starts active, and the page script switches tabs as it does for code groups.
 */
export function ExampleTabs({ tabs, slots }: { tabs: Record<string, string>; slots: Record<string, string> }) {
    const group = useId();
    const entries = Object.entries(tabs);
    return (
        <div className="vp-code-group example-tabs">
            <div className="tabs">
                {entries.map(([slot, label], i) => (
                    <Fragment key={slot}>
                        <input id={`${group}-${i}`} type="radio" name={group} defaultChecked={i === 0} />
                        <label htmlFor={`${group}-${i}`} data-title={label}>
                            {label}
                        </label>
                    </Fragment>
                ))}
            </div>
            <div className="blocks">
                {entries.map(([slot], i) => (
                    <Html key={slot} className={i === 0 ? 'vp-block active' : 'vp-block'} html={slots[slot] ?? ''} />
                ))}
            </div>
        </div>
    );
}
