// Full-size ANSI keyboard for the heatmap, in key units (1u = one letter key).

export interface KeyCap {
  code: string;
  label: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.userAgent);

const MODIFIER_LABELS: Record<string, [mac: string, other: string]> = {
  ControlLeft: ["⌃", "Ctrl"],
  ControlRight: ["⌃", "Ctrl"],
  MetaLeft: ["⌘", "Win"],
  MetaRight: ["⌘", "Win"],
  AltLeft: ["⌥", "Alt"],
  AltRight: ["⌥", "Alt"],
};

/** [code, label, width] runs laid out left to right from `x`. */
type Run = [x: number, y: number, keys: Array<[string, string, number?]>];

const letters = (s: string) => [...s].map((c): [string, string] => [`Key${c}`, c]);
const digits = [..."1234567890"].map((c): [string, string] => [`Digit${c}`, c]);

const RUNS: Run[] = [
  [0, 0, [["Escape", "Esc"]]],
  [2, 0, [["F1", "F1"], ["F2", "F2"], ["F3", "F3"], ["F4", "F4"]]],
  [6.5, 0, [["F5", "F5"], ["F6", "F6"], ["F7", "F7"], ["F8", "F8"]]],
  [11, 0, [["F9", "F9"], ["F10", "F10"], ["F11", "F11"], ["F12", "F12"]]],
  [15.25, 0, [["PrintScreen", "PrtSc"], ["ScrollLock", "ScrLk"], ["Pause", "Pause"]]],

  [0, 1.25, [["Backquote", "`"], ...digits, ["Minus", "-"], ["Equal", "="], ["Backspace", "⌫", 2]]],
  [15.25, 1.25, [["Insert", "Ins"], ["Home", "Home"], ["PageUp", "PgUp"]]],
  [18.5, 1.25, [["NumLock", "Num"], ["NumpadDivide", "/"], ["NumpadMultiply", "*"], ["NumpadSubtract", "-"]]],

  [0, 2.25, [["Tab", "Tab", 1.5], ...letters("QWERTYUIOP"), ["BracketLeft", "["], ["BracketRight", "]"], ["Backslash", "\\", 1.5]]],
  [15.25, 2.25, [["Delete", "Del"], ["End", "End"], ["PageDown", "PgDn"]]],
  [18.5, 2.25, [["Numpad7", "7"], ["Numpad8", "8"], ["Numpad9", "9"]]],

  [0, 3.25, [["CapsLock", "Caps", 1.75], ...letters("ASDFGHJKL"), ["Semicolon", ";"], ["Quote", "'"], ["Enter", "⏎", 2.25]]],
  [18.5, 3.25, [["Numpad4", "4"], ["Numpad5", "5"], ["Numpad6", "6"]]],

  [0, 4.25, [["ShiftLeft", "⇧", 2.25], ...letters("ZXCVBNM"), ["Comma", ","], ["Period", "."], ["Slash", "/"], ["ShiftRight", "⇧", 2.75]]],
  [16.25, 4.25, [["ArrowUp", "↑"]]],
  [18.5, 4.25, [["Numpad1", "1"], ["Numpad2", "2"], ["Numpad3", "3"]]],

  [0, 5.25, [
    ["ControlLeft", "", 1.25], ["MetaLeft", "", 1.25], ["AltLeft", "", 1.25], ["Space", "", 6.25],
    ["AltRight", "", 1.25], ["MetaRight", "", 1.25], ["ContextMenu", "☰", 1.25], ["ControlRight", "", 1.25],
  ]],
  [15.25, 5.25, [["ArrowLeft", "←"], ["ArrowDown", "↓"], ["ArrowRight", "→"]]],
  [18.5, 5.25, [["Numpad0", "0", 2], ["NumpadDecimal", "."]]],
];

/** Keys spanning two rows. */
const TALL: KeyCap[] = [
  { code: "NumpadAdd", label: "+", x: 21.5, y: 2.25, w: 1, h: 2 },
  { code: "NumpadEnter", label: "⏎", x: 21.5, y: 4.25, w: 1, h: 2 },
];

export const KEYBOARD_W = 22.5;
export const KEYBOARD_H = 6.25;

export function keyboardLayout(mac = isMac): KeyCap[] {
  const caps: KeyCap[] = [];
  for (const [x0, y, keys] of RUNS) {
    let x = x0;
    for (const [code, label, w = 1] of keys) {
      const mod = MODIFIER_LABELS[code];
      caps.push({ code, label: mod ? mod[mac ? 0 : 1] : label, x, y, w, h: 1 });
      x += w;
    }
  }
  return [...caps, ...TALL];
}
