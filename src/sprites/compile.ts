import { PALETTE, TRANSPARENT } from "./palette.ts";

export interface SpritePath {
  color: string;
  d: string;
}

/** Merges horizontal runs of each color into one SVG path per color. */
export function compileGrid(rows: readonly string[]): SpritePath[] {
  const byColor = new Map<string, string[]>();
  rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const ch = row[x];
      let end = x + 1;
      while (end < row.length && row[end] === ch) end++;
      if (ch !== TRANSPARENT) {
        const color = PALETTE[ch];
        if (!color) throw new Error(`unknown palette char "${ch}"`);
        const parts = byColor.get(color) ?? [];
        parts.push(`M${x} ${y}h${end - x}v1h${x - end}z`);
        byColor.set(color, parts);
      }
      x = end;
    }
  });
  return [...byColor].map(([color, parts]) => ({ color, d: parts.join("") }));
}
