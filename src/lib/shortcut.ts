// Global shortcuts as the settings hold them: accelerators such as
// "Control+Alt+KeyJ", modifiers first in a fixed order, then the key's W3C
// code. The Rust side parses the same strings.

import { platform, type Platform } from "./platform";

/** Keys a shortcut can end in: the ones the shortcut library knows, less the
 * lock keys and Escape (which cancels the recording). */
const KEY = new RegExp(
  "^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4])|Numpad([0-9]|Add|Decimal|Divide|Enter|Equal|Multiply|Subtract)" +
    "|Arrow(Up|Down|Left|Right)|Backquote|Backslash|BracketLeft|BracketRight|Comma|Equal|Minus|Period|Quote" +
    "|Semicolon|Slash|Space|Tab|Enter|Backspace|Delete|End|Home|Insert|PageDown|PageUp)$",
);

const MODIFIERS = ["Control", "Alt", "Shift", "Super"] as const;
type Modifier = (typeof MODIFIERS)[number];

export interface KeyPress {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

/**
 * The accelerator a key press makes, or null if it cannot be one: the key
 * must be one a shortcut can end in, held with Control, Alt or the
 * Command/Windows key (Shift alone would take a character from typing).
 */
export function fromKeyPress(e: KeyPress): string | null {
  if (!KEY.test(e.code)) return null;
  if (!e.ctrlKey && !e.altKey && !e.metaKey) return null;
  const held: Record<Modifier, boolean> = {
    Control: e.ctrlKey,
    Alt: e.altKey,
    Shift: e.shiftKey,
    Super: e.metaKey,
  };
  return [...MODIFIERS.filter((m) => held[m]), e.code].join("+");
}

const MAC_SYMBOL: Record<Modifier, string> = { Control: "⌃", Alt: "⌥", Shift: "⇧", Super: "⌘" };
const PC_NAME: Record<Modifier, string> = { Control: "Ctrl", Alt: "Alt", Shift: "Shift", Super: "Win" };

const KEY_NAME: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backquote: "`",
  Backslash: "\\",
  BracketLeft: "[",
  BracketRight: "]",
  Comma: ",",
  Equal: "=",
  Minus: "-",
  Period: ".",
  Quote: "'",
  Semicolon: ";",
  Slash: "/",
  PageUp: "PgUp",
  PageDown: "PgDn",
};

function keyName(code: string): string {
  if (KEY_NAME[code]) return KEY_NAME[code];
  const m = /^(?:Key|Digit)(.)$/.exec(code);
  if (m) return m[1];
  if (code.startsWith("Numpad")) return `Num ${code.slice(6)}`;
  return code;
}

/** How a shortcut reads on this platform: "⌃⌥J" on a Mac, "Ctrl+Alt+J" elsewhere. */
export function describeShortcut(accelerator: string, on: Platform = platform): string {
  const parts = accelerator.split("+");
  const key = keyName(parts.pop() ?? "");
  const mods = parts as Modifier[];
  if (on === "mac") return mods.map((m) => MAC_SYMBOL[m]).join("") + key;
  const names = { ...PC_NAME, Super: on === "windows" ? "Win" : "Super" };
  return [...mods.map((m) => names[m]), key].join("+");
}
