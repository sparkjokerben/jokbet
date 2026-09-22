// Dev-only harness: renders the app windows in a plain browser with fake IPC
// data, so layout can be reviewed without the Tauri shell. Not bundled.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { mount } from "svelte";
import App from "../app/App.svelte";
import "../app/theme.css";
import type { Settings } from "../lib/types";

const FREQ = "ETAOINSHRDLCUMWFGYPBVKJXQZ";

function fakeStats(days: number) {
  const today = new Date();
  const out = [];
  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(d.getDate() - i);
    const weekend = d.getDay() === 0 || d.getDay() === 6;
    const keys = Math.round((weekend ? 2500 : 9000) * (0.6 + Math.abs(Math.sin(i * 1.7)) * 0.8));
    out.push({
      date: d.toISOString().slice(0, 10),
      keys,
      clickLeft: Math.round(keys * 0.18),
      clickRight: Math.round(keys * 0.02),
      clickMiddle: Math.round(keys * 0.004),
      scrolls: Math.round(keys * 0.05),
      movePx: keys * 40,
      moveMm: keys * 9,
    });
  }
  const keys: Record<string, number> = { Space: 21000, Backspace: 6400, Enter: 3100, ShiftLeft: 2900, MetaLeft: 2600 };
  [...FREQ].forEach((c, i) => (keys[`Key${c}`] = Math.round(12000 / (i + 1.2))));
  for (let n = 0; n < 10; n++) keys[`Digit${n}`] = 300 + n * 40;
  return { days: out, keys, lifetime: { ...out[0], keys: 1_234_567, moveMm: 12_345_678 } };
}

let settings: Settings = {
  petSize: "medium",
  petPosition: null,
  headCounter: { enabled: true, kind: "today", keyboard: true, mouse: false },
  bubble: true,
  typingSpeed: true,
  milestones: true,
  customMilestones: [{ id: "a", period: "daily", metric: "keys", threshold: 30000, repeat: false }],
  sleepAfterMin: 5,
  paused: false,
  onboarded: true,
};

mockWindows("stats");
mockIPC((cmd, args) => {
  const a = args as Record<string, unknown>;
  switch (cmd) {
    case "get_stats":
      return fakeStats(a.days as number);
    case "get_settings":
      return settings;
    case "update_settings":
      settings = { ...settings, ...(a.patch as object) } as Settings;
      return settings;
    case "plugin:event|listen":
      return 1;
    default:
      return null;
  }
});

mount(App, { target: document.getElementById("root")! });
