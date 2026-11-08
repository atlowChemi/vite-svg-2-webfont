import { Fragment, useId, type FC } from 'react';
import tabs from '../../build/codeTabs.module.css';
import { checkTabCount } from '../../build/codeTabs.ts';
import styles from './ExampleTabs.module.css';

interface Props {
    /** Each slot's tab label, by slot name, in display order. */
    tabs: Record<string, string>;
    /** Each slot's rendered HTML. */
    slots: Record<string, string>;
}

/**
 * Tabs like a code group's, whose panels hold any content, not only code. The first tab starts selected; CSS switches
 * panels as it does for code groups.
 */
export const ExampleTabs: FC<Props> = ({ tabs: labels, slots }) => {
    const group = useId();
    const entries = Object.entries(labels);
    checkTabCount(entries.length, 'An example');
    return (
        <div className={tabs.group}>
            <div className={tabs.tabs}>
                {entries.map(([slot, label], i) => (
                    <Fragment key={slot}>
                        <input id={`${group}-${i}`} type="radio" name={group} defaultChecked={i === 0} />
                        <label htmlFor={`${group}-${i}`} data-title={label}>
                            {label}
                        </label>
                    </Fragment>
                ))}
            </div>
            <div className={tabs.panels}>
                {entries.map(([slot]) => (
                    <div key={slot} className={styles.panel} dangerouslySetInnerHTML={{ __html: slots[slot] ?? '' }} />
                ))}
            </div>
        </div>
    );
};
