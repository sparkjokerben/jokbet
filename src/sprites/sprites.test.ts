import { describe, expect, it } from "vitest";
import { ANIMS, GRID_H, GRID_W, PET_Y, compose, frameRows } from "./jokbet";
import { SOCCER_IDLE_BEFORE } from "./frames/soccer";
import { TYPING_INTRO } from "./frames/typing";
import { compileGrid } from "./compile";
import { PALETTE, TRANSPARENT } from "./palette";

const valid = new Set([TRANSPARENT, ...Object.keys(PALETTE)]);

/** The pet's box in the canvas, as recovered from the reference videos. */
const IDLE = [
  "..........OOOOOOOOOOOOOOOO..............",
  "..........OOOOOOOOOOOOOOOO..............",
  "..........OOEEOOOOOOOOEEOO..............",
  "..........OOEEOOOOOOOOEEOO..............",
  "......OOOOOOOOOOOOOOOOOOOOOOOO..........",
  "......OOOOOOOOOOOOOOOOOOOOOOOO..........",
  "......OOOOOOOOOOOOOOOOOOOOOOOO..........",
  "......OOOOOOOOOOOOOOOOOOOOOOOO..........",
  "..........OOOOOOOOOOOOOOOO..............",
  "..........OOOOOOOOOOOOOOOO..............",
  "..........OOOOOOOOOOOOOOOO..............",
  "..........OOOOOOOOOOOOOOOO..............",
  "..........OO..OO....OO..OO..............",
  "..........OO..OO....OO..OO..............",
  "..........OO..OO....OO..OO..............",
  "..........OO..OO....OO..OO..............",
];

describe("jokbet sprite", () => {
  it("draws the reference's idle pose", () => {
    expect(compose().slice(PET_Y, PET_Y + 16)).toEqual(IDLE);
  });

  it("agrees with the idle frame recovered from the video", () => {
    expect(compose()).toEqual(SOCCER_IDLE_BEFORE[0].rows);
  });

  for (const [name, anim] of Object.entries(ANIMS)) {
    it(`"${name}" frames fit the canvas and use palette colors`, () => {
      expect(anim.frames.length).toBeGreaterThan(0);
      for (const frame of anim.frames) {
        expect(frame.ms).toBeGreaterThan(0);
        const rows = frameRows(frame);
        expect(rows).toHaveLength(GRID_H);
        for (const row of rows) {
          expect(row).toHaveLength(GRID_W);
          for (const ch of row) expect(valid.has(ch)).toBe(true);
        }
      }
    });
  }

  it("keeps the typing intro inside the canvas too", () => {
    for (const frame of TYPING_INTRO) {
      expect(frame.rows).toHaveLength(GRID_H);
      for (const row of frame.rows) expect(row).toHaveLength(GRID_W);
    }
  });
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
