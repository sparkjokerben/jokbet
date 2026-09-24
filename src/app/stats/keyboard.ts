// Keyboards for the heatmap, in key units (1u = one letter key): a PC board
// (full size, ANSI or ISO) on Windows and Linux, and an Apple one on a Mac
// (compact or with the numeric keypad, ANSI or ISO). JIS boards are drawn as
// ANSI.

import type { Platform } from "../../lib/platform";

export interface KeyCap {
  code: string;
  label: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** A cut-out at the bottom left, as the ISO Enter has: its width and height. */
  notch?: { w: number; h: number };
}

export interface Keyboard {
  caps: KeyCap[];
  width: number;
  height: number;
}

export interface LayoutOptions {
  platform: Platform;
  /** ISO: an L-shaped Enter and one more key beside the left Shift. */
  iso: boolean;
  /** Mac only: the Magic Keyboard with a numeric keypad, not the compact one. */
  full: boolean;
}

/** [code, label, width] runs laid out left to right from `x`. */
type Key = [code: string, label: string, w?: number];
type Run = [x: number, y: number, keys: Key[]];

const letters = (s: string) => [...s].map((c): Key => [`Key${c}`, c]);
const digits = [..."1234567890"].map((c): Key => [`Digit${c}`, c]);
const fkeys = (from: number, to: number, w = 1) =>
  Array.from({ length: to - from + 1 }, (_, i): Key => [`F${from + i}`, `F${from + i}`, w]);

/** Rows of the main block, top down; each is 15 units wide. */
const ROW = [1.25, 2.25, 3.25, 4.25, 5.25] as const;

/** The main block's first four rows, which only differ in ISO. */
function typingRows(o: LayoutOptions): { runs: Run[]; extra: KeyCap[] } {
  const mac = o.platform === "mac";
  // On an Apple ISO board the key left of 1 reports IntlBackslash (it is
  // kVK_ISO_Section, labelled §), and Backquote moves beside the left Shift.
  const topLeft: Key = mac && o.iso ? ["IntlBackslash", "§"] : ["Backquote", "`"];
  const besideShift: Key = mac ? ["Backquote", "`"] : ["IntlBackslash", "\\"];
  const backspace: Key = ["Backspace", "⌫", 2];
  const tab: Key = ["Tab", mac ? "⇥" : "Tab", 1.5];
  const caps: Key = ["CapsLock", mac ? "⇪" : "Caps", 1.75];
  const top = [topLeft, ...digits, ["Minus", "-"], ["Equal", "="], backspace] as Key[];
  const qwerty = [tab, ...letters("QWERTYUIOP"), ["BracketLeft", "["], ["BracketRight", "]"]] as Key[];
  const home = [caps, ...letters("ASDFGHJKL"), ["Semicolon", ";"], ["Quote", "'"]] as Key[];
  const bottom = [...letters("ZXCVBNM"), ["Comma", ","], ["Period", "."], ["Slash", "/"]] as Key[];
  if (!o.iso) {
    return {
      runs: [
        [0, ROW[0], top],
        [0, ROW[1], [...qwerty, ["Backslash", "\\", 1.5]]],
        [0, ROW[2], [...home, ["Enter", "⏎", 2.25]]],
        [0, ROW[3], [["ShiftLeft", "⇧", 2.25], ...bottom, ["ShiftRight", "⇧", 2.75]]],
      ],
      extra: [],
    };
  }
  return {
    runs: [
      [0, ROW[0], top],
      [0, ROW[1], qwerty],
      [0, ROW[2], [...home, ["Backslash", mac ? "\\" : "#"]]],
      [0, ROW[3], [["ShiftLeft", "⇧", 1.25], besideShift, ...bottom, ["ShiftRight", "⇧", 2.75]]],
    ],
    // The L-shaped Enter: 1.5u wide on the top row, 1.25u on the home row.
    extra: [{ code: "Enter", label: "⏎", x: 13.5, y: ROW[1], w: 1.5, h: 2, notch: { w: 0.25, h: 1 } }],
  };
}

function pcBoard(o: LayoutOptions): Keyboard {
  const meta = o.platform === "windows" ? "Win" : "Super";
  const { runs, extra } = typingRows(o);
  return build(
    [
      [0, 0, [["Escape", "Esc"]]],
      [2, 0, fkeys(1, 4)],
      [6.5, 0, fkeys(5, 8)],
      [11, 0, fkeys(9, 12)],
      [15.25, 0, [["PrintScreen", "PrtSc"], ["ScrollLock", "ScrLk"], ["Pause", "Pause"]]],
      ...runs,
      [0, ROW[4], [
        ["ControlLeft", "Ctrl", 1.25], ["MetaLeft", meta, 1.25], ["AltLeft", "Alt", 1.25], ["Space", "", 6.25],
        ["AltRight", "Alt", 1.25], ["MetaRight", meta, 1.25], ["ContextMenu", "☰", 1.25], ["ControlRight", "Ctrl", 1.25],
      ]],
      [15.25, ROW[0], [["Insert", "Ins"], ["Home", "Home"], ["PageUp", "PgUp"]]],
      [15.25, ROW[1], [["Delete", "Del"], ["End", "End"], ["PageDown", "PgDn"]]],
      [16.25, ROW[3], [["ArrowUp", "↑"]]],
      [15.25, ROW[4], [["ArrowLeft", "←"], ["ArrowDown", "↓"], ["ArrowRight", "→"]]],
      [18.5, ROW[0], [["NumLock", "Num"], ["NumpadDivide", "/"], ["NumpadMultiply", "*"], ["NumpadSubtract", "-"]]],
      [18.5, ROW[1], [["Numpad7", "7"], ["Numpad8", "8"], ["Numpad9", "9"]]],
      [18.5, ROW[2], [["Numpad4", "4"], ["Numpad5", "5"], ["Numpad6", "6"]]],
      [18.5, ROW[3], [["Numpad1", "1"], ["Numpad2", "2"], ["Numpad3", "3"]]],
      [18.5, ROW[4], [["Numpad0", "0", 2], ["NumpadDecimal", "."]]],
    ],
    [
      ...extra,
      { code: "NumpadAdd", label: "+", x: 21.5, y: ROW[1], w: 1, h: 2 },
      { code: "NumpadEnter", label: "⏎", x: 21.5, y: ROW[3], w: 1, h: 2 },
    ],
    22.5,
  );
}

/** A MacBook's keyboard, or the compact Magic Keyboard. */
function macCompact(o: LayoutOptions): Keyboard {
  const { runs, extra } = typingRows(o);
  return build(
    [
      [0, 0, [["Escape", "esc", 1.5], ...fkeys(1, 12, 1.125)]],
      ...runs,
      [0, ROW[4], [
        ["Fn", "fn"], ["ControlLeft", "⌃"], ["AltLeft", "⌥"], ["MetaLeft", "⌘", 1.25], ["Space", "", 5.5],
        ["MetaRight", "⌘", 1.25], ["AltRight", "⌥"], ["ArrowLeft", "←"],
      ]],
      [14, ROW[4], [["ArrowRight", "→"]]],
    ],
    [
      ...extra,
      // Half-height up and down between left and right.
      { code: "ArrowUp", label: "↑", x: 13, y: ROW[4], w: 1, h: 0.5 },
      { code: "ArrowDown", label: "↓", x: 13, y: ROW[4] + 0.5, w: 1, h: 0.5 },
    ],
    15,
  );
}

/** The Magic Keyboard with Numeric Keypad. */
function macFull(o: LayoutOptions): Keyboard {
  const { runs, extra } = typingRows(o);
  return build(
    [
      [0, 0, [["Escape", "esc"]]],
      [2, 0, fkeys(1, 4)],
      [6.5, 0, fkeys(5, 8)],
      [11, 0, fkeys(9, 12)],
      [15.25, 0, fkeys(13, 15)],
      [18.5, 0, fkeys(16, 19)],
      ...runs,
      [0, ROW[4], [
        ["ControlLeft", "⌃", 1.25], ["AltLeft", "⌥", 1.25], ["MetaLeft", "⌘", 1.5], ["Space", "", 6.75],
        ["MetaRight", "⌘", 1.5], ["AltRight", "⌥", 1.25], ["ControlRight", "⌃", 1.5],
      ]],
      [15.25, ROW[0], [["Fn", "fn"], ["Home", "↖"], ["PageUp", "⇞"]]],
      [15.25, ROW[1], [["Delete", "⌦"], ["End", "↘"], ["PageDown", "⇟"]]],
      [16.25, ROW[3], [["ArrowUp", "↑"]]],
      [15.25, ROW[4], [["ArrowLeft", "←"], ["ArrowDown", "↓"], ["ArrowRight", "→"]]],
      [18.5, ROW[0], [["NumLock", "clear"], ["NumpadEqual", "="], ["NumpadDivide", "/"], ["NumpadMultiply", "*"]]],
      [18.5, ROW[1], [["Numpad7", "7"], ["Numpad8", "8"], ["Numpad9", "9"], ["NumpadSubtract", "-"]]],
      [18.5, ROW[2], [["Numpad4", "4"], ["Numpad5", "5"], ["Numpad6", "6"], ["NumpadAdd", "+"]]],
      [18.5, ROW[3], [["Numpad1", "1"], ["Numpad2", "2"], ["Numpad3", "3"]]],
      [18.5, ROW[4], [["Numpad0", "0", 2], ["NumpadDecimal", "."]]],
    ],
    [...extra, { code: "NumpadEnter", label: "⌤", x: 21.5, y: ROW[3], w: 1, h: 2 }],
    22.5,
  );
}

function build(runs: Run[], extra: KeyCap[], width: number): Keyboard {
  const caps: KeyCap[] = [];
  for (const [x0, y, keys] of runs) {
    let x = x0;
    for (const [code, label, w = 1] of keys) {
      caps.push({ code, label, x, y, w, h: 1 });
      x += w;
    }
  }
  return { caps: [...caps, ...extra], width, height: ROW[4] + 1 };
}

export function keyboardLayout(o: LayoutOptions): Keyboard {
  if (o.platform !== "mac") return pcBoard(o);
  return o.full ? macFull(o) : macCompact(o);
}

/** The rectangles a cap covers: one, or two for a notched cap. */
export function capRects(k: KeyCap): Array<{ x: number; y: number; w: number; h: number }> {
  if (!k.notch) return [{ x: k.x, y: k.y, w: k.w, h: k.h }];
  const top = k.h - k.notch.h;
  return [
    { x: k.x, y: k.y, w: k.w, h: top },
    { x: k.x + k.notch.w, y: k.y + top, w: k.w - k.notch.w, h: k.notch.h },
  ];
}

/** Keys only a Mac keyboard with a numeric keypad has: a compact one sends
 * Home, End, the page keys and forward delete too (fn with the arrows and
 * delete), so those tell nothing. */
const FULL_SIZE_ONLY = /^(Numpad.*|NumLock|F1[3-9])$/;

/** What the system says of the Mac's keyboard (unknown elsewhere). */
export type KeyboardKind = "ansi" | "iso" | "unknown";

/**
 * Which board to draw, from the platform, the keys ever pressed on this
 * machine, and (on a Mac) what the system says the keyboard is. Only keys an
 * ISO or a full-size board has can tip it that way, so the board does not
 * change with the date range being looked at.
 */
export function layoutFor(platform: Platform, everPressed: readonly string[], kind: KeyboardKind): LayoutOptions {
  const pressedIso = everPressed.includes("IntlBackslash");
  if (platform !== "mac") return { platform, iso: pressedIso, full: true };
  return {
    platform,
    iso: kind === "iso" || (kind === "unknown" && pressedIso),
    full: everPressed.some((code) => FULL_SIZE_ONLY.test(code)),
  };
}
