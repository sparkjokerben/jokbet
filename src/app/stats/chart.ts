// Pure chart math shared by the trend chart and the heatmap.

const NICE = [1, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10];

/** Rounds up to a clean number so the axis top and its half land on round ticks. */
export function niceMax(v: number): number {
  if (!(v > 0)) return 1;
  const exp = 10 ** Math.floor(Math.log10(v));
  const f = v / exp;
  return (NICE.find((n) => f <= n + 1e-9) ?? 10) * exp;
}

/** Column with a rounded data end (top) and a square baseline. */
export function columnPath(x: number, y: number, w: number, h: number, r = 4): string {
  if (h <= 0 || w <= 0) return "";
  const rr = Math.min(r, w / 2, h);
  return (
    `M${x} ${y + h}V${y + rr}Q${x} ${y} ${x + rr} ${y}` +
    `H${x + w - rr}Q${x + w} ${y} ${x + w} ${y + rr}V${y + h}Z`
  );
}

export const HEAT_BINS = 8;

/**
 * Heat bin 0..7 for a count, or -1 for zero. Square-root scaling keeps rarely
 * used keys visible next to Space and E.
 */
export function heatBin(count: number, max: number): number {
  if (count <= 0 || max <= 0) return -1;
  return Math.min(HEAT_BINS - 1, Math.floor(Math.sqrt(count / max) * HEAT_BINS));
}

/** Evenly spaced tick indexes for x labels (always includes first and last). */
export function labelIndexes(n: number, max = 5): number[] {
  if (n <= 0) return [];
  if (n <= max) return [...Array(n).keys()];
  const out = new Set<number>();
  for (let i = 0; i < max; i++) out.add(Math.round((i * (n - 1)) / (max - 1)));
  return [...out];
}

/**
 * The column a pointer is over. `left` is the left edge of the area the pointer
 * is measured in — the bounding box of the element the pointer event is bound
 * to — so a caller that passes a hit rectangle's own `rect.left` is already
 * measuring from the first column and must not take the margin off again.
 */
export function columnAt(
  clientX: number,
  left: number,
  band: number,
  count: number,
): number | null {
  if (!(band > 0) || count <= 0) return null;
  const i = Math.floor((clientX - left) / band);
  return i >= 0 && i < count ? i : null;
}
