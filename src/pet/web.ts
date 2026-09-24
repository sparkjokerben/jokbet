// Runs the app's pet inside the website.
//
// The website could have had a GIF; instead it runs this: the same
// PetController, the same sprites, the same counter chip as the app, driven by
// what the visitor does on the page rather than by a global input hook. The
// site imports it as /assets/pet.js (see scripts/build-site-pet.ts), so the pet
// on jokbet.jokerben.top is the pet in the app, not a copy of it.
//
// The site styles .pet-stage, .pet-above and .pet-counter (site/styles.css):
// a `<style>` injected from here would be blocked by the site's CSP.

import { formatCount } from "../lib/format.ts";
import type { Lang } from "../lib/i18n.ts";
import { compileGrid } from "../sprites/compile.ts";
import { ANIMS, GRID_H, GRID_W, PET_H, PET_W, PET_X, PET_Y, frameRows, type AnimName } from "../sprites/jokbet.ts";
import { PetController } from "./controller.ts";
import { attachGestures } from "./gestures.ts";
import type { Activity, Blocked, OneShot } from "./machine.ts";

/** The app's own lift above the pet's box (src/pet/Pet.svelte). */
const ABOVE_LIFT = 8;
/** How far the pointer must leave the pet's middle before the eyes move
 * (src-tauri/src/hover.rs calls this the gaze dead zone). */
const GAZE_DEAD_ZONE_PX = 40;
/** What a "type along" demo types for, and how fast. */
const DEMO_TYPING_MS = 2200;
const DEMO_TYPING_KPS = 5;
/** How long a reduced-motion pose stays up before the pet sits again. */
const STILL_HOLD_MS = 1600;
/** How long the blocked face (no permission) is shown for. */
const BLOCK_HOLD_MS = 4200;
/** Keep-proud gap around the edge of where it may be dragged. */
const EDGE = 8;

const SVG_NS = "http://www.w3.org/2000/svg";

/** The chips beside the hero, and what each one makes the pet do. */
export type Demo = "wave" | "hearts" | "soccer" | "typing" | "sleep" | "celebrate" | "lookAround";

export interface WebPetOptions {
  /** Integer, so every sprite pixel is the same number of screen pixels. */
  scale?: number;
  /** prefers-reduced-motion: no timers, one representative frame per action. */
  still?: boolean;
  /** Let the pointer drag it about; it stays where it is put. */
  draggable?: boolean;
  /** What it may be dragged within; its parent by default. */
  bounds?: () => DOMRect | null;
  /** Show the number above its head. */
  counter?: boolean;
  /** What it does when nothing is happening; the app ships "soccer". */
  idleAnim?: AnimName;
  clickAnim?: OneShot;
  doubleClickAnim?: OneShot;
  /** The app's default is a minute without input. */
  sleepAfterMs?: number;
  lang?: Lang;
}

export interface WebPet {
  /** The visitor typed, clicked, … on the page. */
  input(activity: Activity): void;
  setKeysPerSecond(kps: number): void;
  /** Where the pointer is, in client coordinates: its eyes follow. */
  point(x: number, y: number): void;
  /** The number above its head: keys and clicks counted on this page. */
  setCounter(value: number): void;
  setLang(lang: Lang): void;
  setBlocked(blocked: Blocked | null): void;
  demo(name: Demo): void;
  destroy(): void;
}

/** The gaze the app's Rust reports for a cursor at (x, y): each axis is -1, 0
 * or 1 once the cursor is more than 40 px from the pet's middle. */
export function gazeFor(
  x: number,
  y: number,
  box: { x: number; y: number; w: number; h: number },
): [number, number] {
  const axis = (d: number) => (d < -GAZE_DEAD_ZONE_PX ? -1 : d > GAZE_DEAD_ZONE_PX ? 1 : 0);
  return [axis(x - (box.x + box.w / 2)), axis(y - (box.y + box.h / 2))];
}

/** The frame of an animation that stands in for it when motion is off. */
export function stillFrame(anim: AnimName): string[] {
  const frames = ANIMS[anim].frames;
  return [...frameRows(frames[Math.floor((frames.length - 1) / 2)])];
}

/** Where a box may sit: inside `bounds`, and on the screen. The bottom edge is
 * allowed, since that is where the pet stands; the other three keep a gap. */
export function clampBox(
  box: { left: number; top: number; right: number; bottom: number },
  w: number,
  h: number,
  viewport = { width: window.innerWidth, height: window.innerHeight },
) {
  const left = Math.max(box.left, EDGE);
  const top = Math.max(box.top, EDGE);
  const right = Math.min(box.right, viewport.width - EDGE);
  const bottom = Math.min(box.bottom, viewport.height);
  return {
    minX: left,
    maxX: Math.max(left, right - w),
    minY: top,
    maxY: Math.max(top, bottom - h),
  };
}

export function createPet(host: HTMLElement, options: WebPetOptions = {}): WebPet {
  const scale = options.scale ?? 6;
  const still = options.still ?? false;
  const sleepAfterMs = options.sleepAfterMs ?? 60_000;
  const idleAnim = options.idleAnim ?? "soccer";
  const clickAnim = options.clickAnim ?? "wave";
  const doubleClickAnim = options.doubleClickAnim ?? "hearts";
  const showCounter = options.counter ?? true;

  let lang: Lang = options.lang ?? "zh";

  const stage = document.createElement("div");
  stage.className = "pet-stage";
  stage.style.setProperty("--pet-w", `${GRID_W * scale}px`);
  stage.style.setProperty("--pet-h", `${GRID_H * scale}px`);
  // The chip sits over the pet's middle, not the canvas's (the laptop side is
  // wider), lifted clear of its head — the app's own arithmetic.
  stage.style.setProperty("--pet-above", `${(GRID_H - PET_Y) * scale + ABOVE_LIFT}px`);
  stage.style.setProperty("--pet-shift", `${(PET_X + PET_W / 2 - GRID_W / 2) * scale}px`);

  const above = document.createElement("div");
  above.className = "pet-above";
  const chip = document.createElement("div");
  chip.className = "pet-counter";
  if (showCounter) above.append(chip);

  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("class", "pet-sprite");
  svg.setAttribute("viewBox", `0 0 ${GRID_W} ${GRID_H}`);
  svg.setAttribute("width", String(GRID_W * scale));
  svg.setAttribute("height", String(GRID_H * scale));
  svg.setAttribute("shape-rendering", "crispEdges");
  svg.setAttribute("aria-hidden", "true");

  stage.append(above, svg);
  host.append(stage);

  // One path per colour, kept between frames: only `d` changes.
  const paths = new Map<string, SVGPathElement>();
  let lastKey = "";
  function render(rows: readonly string[]) {
    const key = rows.join("\n");
    if (key === lastKey) return;
    lastKey = key;
    const seen = new Set<string>();
    for (const { color, d } of compileGrid(rows)) {
      seen.add(color);
      let path = paths.get(color);
      if (!path) {
        path = document.createElementNS(SVG_NS, "path");
        path.setAttribute("fill", color);
        paths.set(color, path);
        svg.append(path);
      }
      path.setAttribute("d", d);
    }
    for (const [color, path] of paths) if (!seen.has(color)) path.setAttribute("d", "");
  }

  let value = 0;
  function drawCounter() {
    if (showCounter) chip.textContent = formatCount(value, lang);
  }

  const controller = new PetController(
    (rows) => {
      if (!still) render(rows);
    },
    () => performance.now(),
  );
  controller.setSleepAfter(sleepAfterMs);
  controller.setIdleAnim(idleAnim);

  /** In still mode nothing animates: an action shows its middle frame for a
   * moment, and then the pet is sitting again. */
  let stillTimer: ReturnType<typeof setTimeout> | undefined;
  let stillShown: AnimName | null = null;
  function showStill(anim: AnimName, holdMs = STILL_HOLD_MS) {
    if (stillShown === anim) return;
    stillShown = anim;
    render(stillFrame(anim));
    clearTimeout(stillTimer);
    stillTimer = setTimeout(() => {
      stillShown = null;
      render(stillFrame("idle"));
    }, holdMs);
  }

  /** Sleeping on demand: the app's rule, with the wait set to nothing. The next
   * input wakes it, exactly as it would in the app. */
  let forcedSleep = false;
  function wake() {
    if (!forcedSleep) return;
    forcedSleep = false;
    controller.setSleepAfter(sleepAfterMs);
  }

  let typingTimer: ReturnType<typeof setInterval> | undefined;
  function stopTyping() {
    clearInterval(typingTimer);
    typingTimer = undefined;
  }
  /** A demo types for a while, keystroke by keystroke, so the paws and the
   * typing hold follow the keys the way they do in the app. */
  function demo(name: Demo) {
    stopTyping();
    if (still) return showStill(name);
    switch (name) {
      case "typing":
        controller.setKeysPerSecond(DEMO_TYPING_KPS);
        typingTimer = setInterval(() => {
          wake();
          controller.input("typing");
        }, 1000 / DEMO_TYPING_KPS);
        setTimeout(stopTyping, DEMO_TYPING_MS);
        break;
      case "sleep":
        forcedSleep = true;
        controller.setSleepAfter(0);
        break;
      case "celebrate":
        controller.celebrate(3500);
        break;
      default:
        // includes "lookAround": a one-shot like the rest, so it plays even if
        // the pet was in the middle of something
        controller.oneShot(name);
        break;
    }
  }

  function run(anim: OneShot) {
    wake();
    stopTyping();
    if (still) showStill(anim);
    else controller.oneShot(anim);
  }

  let blockTimer: ReturnType<typeof setTimeout> | undefined;
  function setBlocked(blocked: Blocked | null, holdMs = BLOCK_HOLD_MS) {
    clearTimeout(blockTimer);
    if (still) {
      if (blocked) showStill(blocked, holdMs);
      return;
    }
    controller.setBlocked(blocked);
    // On the page this is a demonstration (the confused face), not a state a
    // visitor could get stuck in, so it lets go on its own.
    if (blocked) blockTimer = setTimeout(() => controller.setBlocked(null), holdMs);
  }

  // --- dragging -------------------------------------------------------------

  const draggable = options.draggable ?? !still;
  const bounds = options.bounds ?? (() => host.getBoundingClientRect());
  // Where the pointer went down. The gesture code reports the start of a drag
  // without the event, so this listener keeps the position for it.
  let pressedAt = { x: 0, y: 0 };
  /** How far it has been moved from where it stands: what `translate` says. */
  let offset = { x: 0, y: 0 };
  let drag: {
    pointer: { x: number; y: number };
    /** The offset the drag started from, so a second drag does not jump. */
    from: { x: number; y: number };
    /** Where it sits with no offset at all, which is what gets clamped. */
    base: { left: number; top: number };
    w: number;
    h: number;
  } | null = null;
  stage.addEventListener("pointerdown", (e) => {
    pressedAt = { x: e.clientX, y: e.clientY };
  });
  const onMove = (e: PointerEvent) => {
    if (!drag) return;
    const range = clampBox(bounds() ?? stage.getBoundingClientRect(), drag.w, drag.h);
    const wantX = drag.from.x + (e.clientX - drag.pointer.x);
    const wantY = drag.from.y + (e.clientY - drag.pointer.y);
    // Clamp the position it would have with no offset, then keep the offset
    // that comes out of it — otherwise a drag that hits an edge would drift.
    offset = {
      x: Math.min(Math.max(drag.base.left + wantX, range.minX), range.maxX) - drag.base.left,
      y: Math.min(Math.max(drag.base.top + wantY, range.minY), range.maxY) - drag.base.top,
    };
    stage.style.translate = `${offset.x}px ${offset.y}px`;
  };
  const onUp = () => {
    if (!drag) return;
    drag = null;
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onUp);
    controller.setDragging(false);
    // It stays where it was put, the way the app's pet stays where you put it.
  };

  // The gestures are always attached — with motion off, a click still shows the
  // pose it would have played — but only a drag that is allowed moves it.
  const detach = attachGestures(stage, {
    press: () => controller.pressed(),
    click: () => run(clickAnim),
    doubleClick: () => run(doubleClickAnim),
    dragStart: () => {
      if (!draggable) return;
      const rect = stage.getBoundingClientRect();
      drag = {
        pointer: { ...pressedAt },
        from: { ...offset },
        // The rect is where it is *with* the offset applied; take that back off
        // to get the place the clamping speaks about.
        base: { left: rect.left - offset.x, top: rect.top - offset.y },
        w: rect.width,
        h: rect.height,
      };
      controller.setDragging(true);
      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
      window.addEventListener("pointercancel", onUp);
    },
    context: () => run("poke"),
  });

  drawCounter();
  if (still) render(stillFrame("idle"));

  return {
    input(activity) {
      wake();
      if (still) {
        showStill(activity === "typing" ? "typing" : "click");
        return;
      }
      controller.input(activity);
    },
    setKeysPerSecond: (kps) => controller.setKeysPerSecond(kps),
    point(x, y) {
      if (still) return;
      const box = svg.getBoundingClientRect();
      if (!box.width) return;
      controller.setGaze(
        gazeFor(x, y, { x: box.x + PET_X * scale, y: box.y + PET_Y * scale, w: PET_W * scale, h: PET_H * scale }),
      );
    },
    setCounter(next) {
      if (next === value) return;
      value = next;
      drawCounter();
    },
    setLang(next) {
      lang = next;
      drawCounter();
    },
    setBlocked,
    demo,
    destroy() {
      clearTimeout(stillTimer);
      clearTimeout(blockTimer);
      stopTyping();
      detach?.();
      controller.destroy();
      stage.remove();
    },
  };
}
