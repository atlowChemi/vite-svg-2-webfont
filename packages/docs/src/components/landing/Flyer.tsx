import { Wordmark } from '../Wordmark.tsx';
import { emptySurface } from './pour.ts';

/**
 * The hero wordmark that pours and then docks into the top bar. It is fixed and hidden until the landing script has
 * measured where it starts and where it lands; the copy in the hero's placeholder keeps its place until then.
 */
export function Flyer() {
    return (
        <div className="landing-flyer" aria-hidden="true">
            <Wordmark surface={emptySurface} />
        </div>
    );
}
