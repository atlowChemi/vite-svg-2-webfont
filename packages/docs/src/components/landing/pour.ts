// The hero's pour, shared by the build (the first frame) and the landing script (every frame after it).

/** The T's metal level in its 100×86 box: empty at the stem's foot, full at the crossbar. */
export const EMPTY = 90;
export const FULL = 22;

/** The molten surface at `level`, as a path across the T's box: two sines, so the swell never repeats exactly. */
export function wave(level: number, amplitude: number, phase: number): string {
    const points: string[] = [];
    for (let x = 0; x <= 100; x += 4) {
        const y = level + amplitude * Math.sin((x / 25) * Math.PI + phase) + amplitude * 0.5 * Math.sin((x / 16.7) * Math.PI - phase * 1.3);
        points.push(`${x ? 'L' : 'M'}${x} ${y.toFixed(2)}`);
    }
    return points.join(' ');
}

/** The still, empty mold the page loads with. */
export const emptySurface = wave(EMPTY, 0, 0);

/** The rails' temperature, from cold metal (`t` 0) to molten (`t` 1). */
export const degrees = (t: number) => `${Math.round(20 + t * 320)}°C`;
