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

  it("stretches up from the hips, face and arms going with it", () => {
    const tall = compose({ stretch: 2 });
    // The torso's top row is two above where it normally is, over the same columns.
    expect(tall[PET_Y - 2]).toBe(IDLE[0]);
    expect(tall[PET_Y - 1]).toBe(IDLE[0]);
    // The eyes are two rows higher, the feet where they were.
    expect(tall[PET_Y]).toBe(IDLE[2]);
    expect(tall.slice(PET_Y + 12, PET_Y + 16)).toEqual(IDLE.slice(12));
    // Arms reaching up stand above the head.
    const reach = compose({ armL: "reach", armR: "reach" });
    expect(reach[PET_Y - 3].slice(6, 10)).toBe("OOOO");
    expect(reach[PET_Y - 3].slice(26, 30)).toBe("OOOO");
  });

  it("draws a mouth and a tear on the face", () => {
    const at = (rows: string[], x: number, y: number) => rows[PET_Y + y][6 + x];
    const yawn = compose({ mouth: "yawn", tear: true });
    for (let y = 5; y < 8; y++) for (let x = 10; x < 14; x++) expect(at(yawn, x, y)).toBe("E");
    expect(at(yawn, 9, 5)).toBe("O");
    expect(at(yawn, 5, 4)).toBe("S");
    expect(at(yawn, 5, 5)).toBe("S");
    const o = compose({ mouth: "o" });
    expect(at(o, 11, 6) + at(o, 12, 6)).toBe("EE");
    expect(at(o, 11, 5)).toBe("O");
  });

  it("walks with at least two feet on the ground and none tangled", () => {
    for (const frame of ANIMS.walk.frames) {
      for (const dir of [1, -1]) {
        const pose = frame.pose ?? {};
        const legDx = pose.legDx?.map((d) => d * dir) as [number, number, number, number] | undefined;
        const rows = compose({ ...pose, legDx });
        expect(rows[GRID_H - 1].match(/OO/g)?.length).toBeGreaterThanOrEqual(2);
        // Every leg is its own column pair, apart from its neighbours.
        const hip = rows[GRID_H - 3];
        expect(hip.match(/OO/g)).toHaveLength(4);
        expect(hip).not.toMatch(/OOO/);
      }
    }
  });

  it("shifts a leg sideways by its legDx", () => {
    const base = compose();
    const moved = compose({ legDx: [1, 0, 0, 0] });
    const hip = GRID_H - 2;
    expect(base[hip].slice(10, 13)).toBe("OO.");
    expect(moved[hip].slice(10, 13)).toBe(".OO");
  });

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
