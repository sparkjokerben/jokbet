// Dev-only harness: renders the app windows in a plain browser with fake IPC
// data, so layout can be reviewed without the Tauri shell. Not bundled.

import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { mount } from "svelte";
import App from "../app/App.svelte";
import Pet from "../pet/Pet.svelte";
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
  idleAnim: "breathe",
  clickAnim: "poke",
  doubleClickAnim: "hearts",
  bubble: true,
  typingSpeed: true,
  milestones: true,
  customMilestones: [{ id: "a", period: "daily", metric: "keys", threshold: 30000, repeat: false }],
  sleepAfterMin: 5,
  paused: false,
  onboarded: true,
};

const view = new URLSearchParams(location.search).get("view");

mockWindows(view === "pet-frame" ? "pet" : "stats");
mockIPC(
  (cmd, args) => {
  const a = args as Record<string, unknown>;
  switch (cmd) {
    case "get_stats":
      return fakeStats(a.days as number);
    case "get_settings":
      return settings;
    case "get_status":
      return { permission: "denied", listening: false, secureInput: false, paused: false };
    case "plugin:autostart|is_enabled":
      return true;
    case "update_settings":
      settings = { ...settings, ...(a.patch as object) } as Settings;
      return settings;
    default:
      return null;
  }
  },
  { shouldMockEvents: true },
);

if (view === "pet") {
  // A frame the size of the real pet window, over a mid-gray "desktop".
  document.body.style.cssText = "margin:0;background:#8a8d93";
  const frame = document.createElement("iframe");
  frame.src = "/preview.html?view=pet-frame";
  frame.style.cssText = "width:220px;height:266px;border:1px dashed #555;margin:20px";
  document.body.append(frame);
} else if (view === "pet-frame") {
  // pet.html has no theme; undo the one imported for the app views.
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
  mount(Pet, { target: document.getElementById("root")! });
  setTimeout(async () => {
    const today = fakeStats(1).days[0];
    await emit("app://status", { permission: "granted", listening: true, secureInput: false, paused: false });
    await emit("pet://tick", { today, kpm: 186, cpm: 12, activity: null });
    await emit("pet://hover", true);
    await emit("pet://celebrate", {
      hits: [{ id: "a", period: "daily", metric: "keys", level: 10000 }],
    });
  }, 300);
} else {
  mount(App, { target: document.getElementById("root")! });
}
