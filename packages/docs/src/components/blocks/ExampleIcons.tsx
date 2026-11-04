import { iconsFor } from './exampleIcons.ts';
import { Html } from './Html.tsx';

/** The input SVGs of one example, as a thumbnail grid labelled with their path inside `icons/`. */
export function ExampleIcons({ example }: { example: string }) {
    return (
        <div className="example-icons">
            {iconsFor(example).map(icon => (
                <figure key={icon.file}>
                    <Html className="example-icons-svg" html={icon.svg} />
                    <figcaption>{icon.file}</figcaption>
                </figure>
            ))}
        </div>
    );
}
