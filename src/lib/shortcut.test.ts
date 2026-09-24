import { describe, expect, it } from "vitest";
import { describeShortcut, fromKeyPress, type KeyPress } from "./shortcut";

const press = (code: string, mods: Partial<KeyPress> = {}): KeyPress => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("fromKeyPress", () => {
  it("puts modifiers in a fixed order before the key", () => {
    expect(fromKeyPress(press("KeyJ", { metaKey: true, shiftKey: true, ctrlKey: true, altKey: true }))).toBe(
      "Control+Alt+Shift+Super+KeyJ",
    );
    expect(fromKeyPress(press("Digit1", { altKey: true }))).toBe("Alt+Digit1");
  });

  it("needs Control, Alt or Command/Windows", () => {
    expect(fromKeyPress(press("KeyJ"))).toBeNull();
    expect(fromKeyPress(press("KeyJ", { shiftKey: true }))).toBeNull();
  });

  it("refuses keys a shortcut cannot end in", () => {
    for (const code of ["ShiftLeft", "MetaLeft", "Escape", "CapsLock", "Fn", "IntlBackslash"]) {
      expect(fromKeyPress(press(code, { ctrlKey: true })), code).toBeNull();
    }
    expect(fromKeyPress(press("F13", { ctrlKey: true }))).toBe("Control+F13");
    expect(fromKeyPress(press("NumpadAdd", { ctrlKey: true }))).toBe("Control+NumpadAdd");
  });
});

describe("describeShortcut", () => {
  it("uses symbols on a Mac and names elsewhere", () => {
    expect(describeShortcut("Control+Alt+Shift+Super+KeyJ", "mac")).toBe("⌃⌥⇧⌘J");
    expect(describeShortcut("Control+Alt+KeyJ", "windows")).toBe("Ctrl+Alt+J");
    expect(describeShortcut("Super+ArrowUp", "windows")).toBe("Win+↑");
    expect(describeShortcut("Super+Digit2", "linux")).toBe("Super+2");
    expect(describeShortcut("Control+Numpad5", "linux")).toBe("Ctrl+Num 5");
  });
});
