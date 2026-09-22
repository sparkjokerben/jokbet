import { describe, expect, it } from "vitest";
import { ANIMS, GRID_H, GRID_W, compose } from "./clawd";
import { compileGrid } from "./compile";
import { PALETTE, TRANSPARENT } from "./palette";

const valid = new Set([TRANSPARENT, ...Object.keys(PALETTE)]);

describe("clawd sprite", () => {
  it("matches the Claude Code logo in the idle pose", () => {
    expect(compose().slice(6)).toEqual([
      "......OOOOOOOOOOOO......",
      "......OOOOOOOOOOOO......",
      "......OOEOOOOOOEOO......",
      "......OOEOOOOOOEOO......",
      "....OOOOOOOOOOOOOOOO....",
      "....OOOOOOOOOOOOOOOO....",
      "......OOOOOOOOOOOO......",
      "......OOOOOOOOOOOO......",
      ".......O.O....O.O.......",
      ".......O.O....O.O.......",
    ]);
  });

  for (const [name, anim] of Object.entries(ANIMS)) {
    it(`"${name}" frames fit the grid and use palette colors`, () => {
      expect(anim.frames.length).toBeGreaterThan(0);
      for (const frame of anim.frames) {
        expect(frame.ms).toBeGreaterThan(0);
        const rows = compose(frame.pose);
        expect(rows).toHaveLength(GRID_H);
        for (const row of rows) {
          expect(row).toHaveLength(GRID_W);
          for (const ch of row) expect(valid.has(ch)).toBe(true);
        }
      }
    });
  }
});

describe("compileGrid", () => {
  it("merges horizontal runs into one path per color", () => {
    expect(compileGrid(["OO.E", ".OO."])).toEqual([
      { color: PALETTE.O, d: "M0 0h2v1h-2zM1 1h2v1h-2z" },
      { color: PALETTE.E, d: "M3 0h1v1h-1z" },
    ]);
  });

  it("rejects unknown palette characters", () => {
    expect(() => compileGrid(["?"])).toThrow();
  });
});
