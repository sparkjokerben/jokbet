import { describe, expect, it } from "vitest";
import { capRects, keyboardLayout, layoutFor, type LayoutOptions } from "./keyboard";

const VARIANTS: Array<[string, LayoutOptions]> = [
  ["PC ANSI", { platform: "windows", iso: false, full: true }],
  ["PC ISO", { platform: "linux", iso: true, full: true }],
  ["Mac compact ANSI", { platform: "mac", iso: false, full: false }],
  ["Mac compact ISO", { platform: "mac", iso: true, full: false }],
  ["Mac full ANSI", { platform: "mac", iso: false, full: true }],
  ["Mac full ISO", { platform: "mac", iso: true, full: true }],
];

const codes = (o: LayoutOptions) => keyboardLayout(o).caps.map((k) => k.code);

describe.each(VARIANTS)("%s keyboard", (_, options) => {
  const board = keyboardLayout(options);
  const rects = board.caps.flatMap((k) => capRects(k).map((r) => ({ ...r, code: k.code })));

  it("has every key once", () => {
    const all = board.caps.map((k) => k.code);
    expect(new Set(all).size).toBe(all.length);
  });

  it("stays inside the board", () => {
    for (const r of rects) {
      expect(r.x + r.w, r.code).toBeLessThanOrEqual(board.width + 1e-9);
      expect(r.y + r.h, r.code).toBeLessThanOrEqual(board.height + 1e-9);
    }
  });

  it("has main block rows 15 units wide", () => {
    for (const y of [1.25, 2.25, 3.25, 4.25, 5.25]) {
      const row = rects.filter((r) => r.y <= y && y < r.y + r.h && r.x < 15);
      expect(Math.max(...row.map((r) => r.x + r.w)), `row ${y}`).toBeCloseTo(15);
      const width = row.reduce((a, r) => a + r.w, 0);
      expect(width, `row ${y} leaves no gaps`).toBeCloseTo(15);
    }
  });

  it("has no two keys overlapping", () => {
    for (const a of rects) {
      for (const b of rects) {
        if (a === b || a.code === b.code) continue;
        const overlap = a.x < b.x + b.w - 1e-9 && b.x < a.x + a.w - 1e-9 && a.y < b.y + b.h - 1e-9 && b.y < a.y + a.h - 1e-9;
        expect(overlap, `${a.code} overlaps ${b.code}`).toBe(false);
      }
    }
  });
});

describe("keyboard variants", () => {
  it("keeps the full-size PC board at 104 keys, 105 with ISO", () => {
    expect(codes(VARIANTS[0][1])).toHaveLength(104);
    expect(codes(VARIANTS[1][1])).toHaveLength(105);
  });

  it("names modifiers as each platform prints them", () => {
    const label = (o: LayoutOptions, code: string) => keyboardLayout(o).caps.find((k) => k.code === code)?.label;
    expect(label({ platform: "windows", iso: false, full: true }, "MetaLeft")).toBe("Win");
    expect(label({ platform: "linux", iso: false, full: true }, "MetaLeft")).toBe("Super");
    expect(label({ platform: "mac", iso: false, full: false }, "MetaLeft")).toBe("⌘");
    expect(label({ platform: "mac", iso: false, full: false }, "AltLeft")).toBe("⌥");
  });

  it("leaves the keypad and editing keys off a compact Mac board", () => {
    const compact = codes({ platform: "mac", iso: false, full: false });
    expect(compact).toContain("Fn");
    for (const code of ["Numpad5", "Home", "F13", "PrintScreen", "ContextMenu"]) expect(compact).not.toContain(code);
    const full = codes({ platform: "mac", iso: false, full: true });
    for (const code of ["Numpad5", "NumpadEqual", "Home", "F19", "Fn"]) expect(full).toContain(code);
    expect(full).not.toContain("PrintScreen");
  });

  it("swaps the ISO keys as an Apple board does", () => {
    const at = (o: LayoutOptions, code: string) => keyboardLayout(o).caps.find((k) => k.code === code);
    const mac: LayoutOptions = { platform: "mac", iso: true, full: false };
    const pc: LayoutOptions = { platform: "windows", iso: true, full: true };
    expect(at(mac, "IntlBackslash")).toMatchObject({ x: 0, y: 1.25 });
    expect(at(mac, "Backquote")).toMatchObject({ x: 1.25, y: 4.25 });
    expect(at(pc, "Backquote")).toMatchObject({ x: 0, y: 1.25 });
    expect(at(pc, "IntlBackslash")).toMatchObject({ x: 1.25, y: 4.25 });
    expect(at(pc, "Enter")?.notch).toBeDefined();
  });
});

describe("layoutFor", () => {
  it("takes ISO on a PC from its extra key having been pressed", () => {
    expect(layoutFor("windows", [], "unknown")).toEqual({ platform: "windows", iso: false, full: true });
    expect(layoutFor("linux", ["KeyA", "IntlBackslash"], "unknown").iso).toBe(true);
  });

  it("takes ISO on a Mac from the system, with JIS drawn as ANSI", () => {
    expect(layoutFor("mac", [], "iso").iso).toBe(true);
    expect(layoutFor("mac", ["IntlBackslash"], "ansi").iso).toBe(false);
    expect(layoutFor("mac", ["IntlBackslash"], "unknown").iso).toBe(true);
  });

  it("draws the full Mac board only for keys a compact one lacks", () => {
    expect(layoutFor("mac", ["Home", "End", "PageUp", "Delete"], "ansi").full).toBe(false);
    expect(layoutFor("mac", ["Numpad3"], "ansi").full).toBe(true);
    expect(layoutFor("mac", ["F15"], "ansi").full).toBe(true);
    expect(layoutFor("mac", ["F12"], "ansi").full).toBe(false);
  });
});
