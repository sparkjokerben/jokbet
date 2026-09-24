// Dev-only harness: renders the app windows in a plain browser with fake IPC
// data, so layout can be reviewed without the Tauri shell. Not bundled.

import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { mount } from "svelte";
import App from "../app/App.svelte";
import Pet from "../pet/Pet.svelte";
import "../app/theme.css";
import { setLang } from "../lib/i18n";
import type { Settings } from "../lib/types";

const params = new URLSearchParams(location.search);

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
  // ?iso and ?keypad make the keys that tip the heatmap to those boards.
  const everPressed = Object.keys(keys);
  if (params.has("iso")) everPressed.push("IntlBackslash");
  if (params.has("keypad")) everPressed.push("Numpad5");
  return { days: out, keys, lifetime: { ...out[0], keys: 1_234_567, moveMm: 12_345_678 }, everPressed };
}

// Mirrors the shipped defaults (see Settings::default in src-tauri).
let settings: Settings = {
  petScale: 3.5,
  petPosition: null,
  headCounter: { enabled: true, kind: "today", keyboard: true, mouse: true },
  idleAnim: "soccer",
  clickAnim: "wave",
  doubleClickAnim: "hearts",
  bubble: true,
  liquidGlass: true,
  glassTint: 18,
  typingSpeed: true,
  milestones: true,
  customMilestones: [{ id: "a", period: "daily", metric: "keys", threshold: 30000, repeat: false }],
  sleepAfterMin: 1,
  paused: false,
  onboarded: true,
  language: "system",
  hideInFullscreen: true,
  shortcuts: { togglePet: "Control+Alt+KeyJ", pause: null },
  heatmapPalette: params.get("palette") === "heat" ? "heat" : "brand",
};

const view = params.get("view");
/** With ?nostorage, the database "could not be opened". */
const storage = !params.has("nostorage");
/** An update as the updater reports one once downloaded. */
const READY = {
  state: "ready",
  version: "0.2.0",
  notes: "- Hide the pet while another app is full screen\n- Global shortcuts\n- Choose the interface language",
  date: "2026-09-25",
};
/** With ?update, one is downloaded already. */
const update = params.has("update") ? READY : { state: "idle" };
/** With ?theme=light or ?theme=dark, that theme whatever the system's. */
const previewTheme = params.get("theme");
if (previewTheme === "light" || previewTheme === "dark") document.documentElement.dataset.theme = previewTheme;
/** With ?lang=en or ?lang=zh, the pages speak that language. */
const previewLang = params.get("lang");
if (previewLang === "en" || previewLang === "zh") setLang(previewLang);

mockWindows(view === "pet-frame" ? "pet" : "stats");
mockIPC(
  (cmd, args) => {
  const a = args as Record<string, unknown>;
  switch (cmd) {
    case "get_stats":
      return fakeStats(a.days as number);
    case "get_settings":
      return settings;
    case "glass_support":
      return "liquidGlass";
    case "set_glass_bubble": {
      // Where the page thinks the bubble is; drawn on screen for a look.
      const r = a.rect as [number, number, number, number] | null;
      const tag = document.getElementById("glass-rect");
      if (tag) {
        tag.textContent = r ? r.map((v) => Math.round(v)).join(" ") : "none";
        tag.style.display = r ? "block" : "none";
        if (r) {
          tag.style.left = `${r[0]}px`;
          tag.style.top = `${r[1]}px`;
          tag.style.width = `${r[2]}px`;
          tag.style.height = `${r[3]}px`;
          tag.style.borderRadius = `${a.radius}px`;
        }
      }
      return null;
    }
    case "get_status":
      return { permission: "denied", listening: false, paused: false, storage };
    case "plugin:autostart|is_enabled":
      return true;
    case "keyboard_kind":
      return params.has("iso") ? "iso" : "ansi";
    case "plugin:app|version":
      return "0.1.2";
    case "update_status":
      return update;
    case "check_update":
      return READY;
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
  document.body.style.cssText = "margin:0;background:#8a8d93;position:relative";
  const frame = document.createElement("iframe");
  const pass = new URLSearchParams(params);
  pass.set("view", "pet-frame");
  frame.src = `/preview.html?${pass}`;
  // The size of the real pet window at the scale in the fake settings below.
  frame.style.cssText = "width:220px;height:271px;border:1px dashed #555;margin:20px";
  document.body.append(frame);
} else if (view === "pet-frame") {
  // pet.html has no theme; undo the one imported for the app views.
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
  // With ?glassdebug, outlines the reported glass rect: the native material is
  // placed from it, so a wrong rect is worth seeing.
  if (params.has("glassdebug")) {
    const tag = document.createElement("div");
    tag.id = "glass-rect";
    tag.style.cssText =
      "position:fixed;border:1px solid #0f0;color:#0f0;font:11px monospace;" +
      "background:rgba(0,255,0,0.08);pointer-events:none;display:none";
    document.body.append(tag);
  }
  mount(Pet, { target: document.getElementById("root")! });
  setTimeout(async () => {
    const today = fakeStats(1).days[0];
    await emit("app://status", { permission: "granted", listening: true, paused: false, storage });
    await emit("pet://tick", { today, kps: 3.1, cps: 0.2, activity: null });
    await emit("pet://hover", true);
    await emit("pet://celebrate", {
      hits: [{ id: "a", period: "daily", metric: "keys", level: 10000 }],
    });
  }, 300);
} else {
  mount(App, { target: document.getElementById("root")! });
}
