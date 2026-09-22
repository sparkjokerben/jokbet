import { describe, expect, it } from "vitest";
import { KEYBOARD_H, KEYBOARD_W, keyboardLayout } from "./keyboard";

describe("keyboard layout", () => {
  const caps = keyboardLayout(false);

  it("has every key once", () => {
    const codes = caps.map((k) => k.code);
    expect(new Set(codes).size).toBe(codes.length);
    expect(codes.length).toBe(104);
  });

  it("stays inside the board", () => {
    for (const k of caps) {
      expect(k.x + k.w).toBeLessThanOrEqual(KEYBOARD_W + 1e-9);
      expect(k.y + k.h).toBeLessThanOrEqual(KEYBOARD_H + 1e-9);
    }
  });

  it("main block rows are 15 units wide", () => {
    for (const y of [1.25, 2.25, 3.25, 4.25, 5.25]) {
      const right = Math.max(...caps.filter((k) => k.y === y && k.x < 15).map((k) => k.x + k.w));
      expect(right).toBe(15);
    }
  });

  it("no two keys overlap", () => {
    for (const a of caps) {
      for (const b of caps) {
        if (a === b) continue;
        const overlap = a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
        expect(overlap, `${a.code} overlaps ${b.code}`).toBe(false);
      }
    }
  });

  it("labels modifiers per platform", () => {
    expect(keyboardLayout(true).find((k) => k.code === "MetaLeft")?.label).toBe("⌘");
    expect(caps.find((k) => k.code === "MetaLeft")?.label).toBe("Win");
  });
});
