// The parts of the website's pet that can be pinned down without a browser:
// where the eyes look, what a reduced-motion pose is, and how far the pet may
// be dragged.

import { describe, expect, it } from "vitest";
import { compileGrid } from "../sprites/compile.ts";
import { GRID_H, GRID_W } from "../sprites/jokbet.ts";
import { clampBox, gazeFor, stillFrame } from "./web.ts";

const pet = { x: 100, y: 100, w: 120, h: 80 };

describe("the pet's gaze", () => {
  it("stays put while the pointer is near its middle", () => {
    expect(gazeFor(160, 140, pet)).toEqual([0, 0]);
    expect(gazeFor(120, 100, pet)).toEqual([0, 0]); // 40 px out is still in
  });

  it("follows the pointer once it is well past the middle", () => {
    expect(gazeFor(20, 140, pet)).toEqual([-1, 0]);
    expect(gazeFor(300, 140, pet)).toEqual([1, 0]);
    expect(gazeFor(160, 400, pet)).toEqual([0, 1]);
  });

  it("looks diagonally when the pointer is off in both axes", () => {
    expect(gazeFor(0, 0, pet)).toEqual([-1, -1]);
    expect(gazeFor(400, 400, pet)).toEqual([1, 1]);
  });
});

describe("the reduced-motion pose", () => {
  it("is a whole frame of the sprite grid", () => {
    for (const anim of ["idle", "wave", "hearts", "soccer", "typing", "celebrate", "noperm"] as const) {
      const rows = stillFrame(anim);
      expect(rows, anim).toHaveLength(GRID_H);
      for (const row of rows) expect(row, anim).toHaveLength(GRID_W);
      // Every character must be one the palette knows, or drawing throws.
      expect(() => compileGrid(rows), anim).not.toThrow();
    }
  });

  it("shows something, not a blank grid", () => {
    const rows = stillFrame("wave");
    expect(rows.join("").replaceAll(".", "")).not.toBe("");
    expect(rows).not.toEqual(stillFrame("idle"));
  });
});

describe("the drag bounds", () => {
  const viewport = { width: 800, height: 600 };

  it("keeps the pet inside the column it may move in", () => {
    const range = clampBox({ left: 500, top: 0, right: 760, bottom: 4000 }, 240, 156, viewport);
    expect(range.minX).toBe(500);
    expect(range.maxX).toBe(520); // 760 - 240
  });

  it("never lets the pet off the screen, whatever the column says", () => {
    const range = clampBox({ left: -100, top: -100, right: 4000, bottom: 4000 }, 240, 156, viewport);
    expect(range.minX).toBe(8);
    expect(range.maxX).toBe(792 - 240);
    expect(range.minY).toBe(8);
    // The bottom edge is allowed: that is where the pet stands.
    expect(range.maxY).toBe(600 - 156);
  });

  it("lets the pet that stands on the bottom edge be lifted from it", () => {
    const standing = 600 - 156;
    const range = clampBox({ left: 500, top: 0, right: 740, bottom: 4000 }, 240, 156, viewport);
    expect(range.minY).toBeLessThan(standing);
    expect(range.maxY).toBeGreaterThanOrEqual(standing);
  });

  it("allows no movement rather than a negative range", () => {
    const range = clampBox({ left: 0, top: 0, right: 100, bottom: 100 }, 240, 156, viewport);
    expect(range.maxX).toBeGreaterThanOrEqual(range.minX);
    expect(range.maxY).toBeGreaterThanOrEqual(range.minY);
  });
});
