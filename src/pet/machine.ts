// Pure animation state selection and frame timing for the pet.

import type { Anim, AnimName } from "../sprites/jokbet";

export type Blocked = "noperm";
/** Input the pet acts out. */
export type Activity = "typing" | "click";
/** Any input: a scroll keeps the pet awake without it doing anything. */
export type Input = Activity | "scroll";
/** Animations the controller can play once and end. The ones a user may pick for
 * a click or double-click are a narrower set (ActionAnim, src/lib/types.ts). */
export type OneShot =
  | "poke"
  | "hearts"
  | "soccer"
  | "wave"
  | "wake"
  | "typingEnd"
  | "lookAround"
  | "stretch";

/** What the pet may do now and then while idle, between breaths. */
export type IdleChoice = "soccer" | "lookAround" | "walk";

/** A stroll under way: which one (the app numbers them), which way, and
 * whether it is still looking where it is going before it sets off. */
export interface Walking {
  id: number;
  dir: -1 | 1;
  glance: boolean;
  /** How long a frame of the walk lasts at this speed; see walkFrameMs. */
  frameMs: number;
}

export interface Signals {
  blocked: Blocked | null;
  dragging: boolean;
  celebrateUntil: number;
  oneShot: { anim: OneShot; until: number } | null;
  /** The last key or click: what the pet reacts to. */
  lastInputAt: number;
  /** The last scroll, which only keeps it awake. */
  lastScrollAt: number;
  lastActivity: Activity | null;
  /** How long the last activity keeps the pet reacting; see typingReactMs. */
  reactMs: number;
  sleepAfterMs: number;
  /** Out for a stroll, which anything else comes before. */
  walk: Walking | null;
}

/**
 * Why the pet cannot see input: no permission, or no hook. macOS Secure Input
 * is not a reason: it hides only ordinary keys, while clicks, the mouse and the
 * modifier keys still arrive, and a background app can leave it on for hours.
 */
export function blockedBy(s: { permission: string; listening: boolean }): Blocked | null {
  if (s.permission === "denied" || s.permission === "unsupported" || !s.listening) return "noperm";
  return null;
}

/** How long a reaction lasts after the last input of that kind. Keystrokes
 * are special: see typingReactMs. It is nearly as long as getting the laptop
 * out takes, so that the pet is never cut off half-way through. */
export const REACT_MS: Record<Activity, number> = { typing: 1000, click: 300 };

/** The longest the pet stays in typing after the last keystroke. */
export const TYPING_HOLD_MAX_MS = 6000;
/** Extra hold per sqrt(second) of the current spell of typing. */
const TYPING_HOLD_GROWTH = 400;

/**
 * How long keystrokes keep the pet in typing: the base time plus a term that
 * grows with the square root of how long this spell of typing has lasted. A
 * few keys tapped now and then end at once; a long session earns a slightly
 * longer pause before the laptop goes away.
 */
export function typingReactMs(sessionMs: number): number {
  const seconds = Math.max(0, sessionMs) / 1000;
  return Math.min(TYPING_HOLD_MAX_MS, REACT_MS.typing + TYPING_HOLD_GROWTH * Math.sqrt(seconds));
}

/** When the pet last saw anyone: a key, a click or a scroll. */
const lastSeen = (s: Signals) => Math.max(s.lastInputAt, s.lastScrollAt);

/** Highest priority first: blocked > dragged > celebrate > one-shot > sleep >
 * react > walk > idle. */
export function pickAnim(s: Signals, now: number): AnimName {
  if (s.blocked) return s.blocked;
  if (s.dragging) return "dragged";
  if (now < s.celebrateUntil) return "celebrate";
  if (s.oneShot && now < s.oneShot.until) return s.oneShot.anim;
  if (now - lastSeen(s) >= s.sleepAfterMs) return "sleep";
  if (s.lastActivity && now - s.lastInputAt < s.reactMs) return s.lastActivity;
  if (s.walk) return "walk";
  return "idle";
}

/** A walk frame's length: one sprite cell of ground a frame, whatever the
 * speed (logical pixels a second) and the size, within what still reads as
 * walking. */
export function walkFrameMs(speed: number, scale: number): number {
  return Math.min(250, Math.max(60, Math.round((1000 * scale) / Math.max(speed, 1))));
}

/** Where a stroll goes, and how long it takes: a point anywhere along the
 * line the pet stands on (0 its left end, 1 its right), and 3 to 15 seconds.
 * The way to go and the speed follow from where the pet is. */
export interface WalkPlan {
  target: number;
  seconds: number;
}

export function walkPlan(random: () => number = Math.random): WalkPlan {
  return { target: random(), seconds: 3 + 12 * random() };
}

/** One of the chosen idle animations, at random. */
export function pickIdle(choices: readonly IdleChoice[], random: () => number = Math.random): IdleChoice | null {
  if (!choices.length) return null;
  return choices[Math.min(choices.length - 1, Math.floor(random() * choices.length))];
}

/** Earliest future time at which pickAnim may change without new signals. */
export function nextDeadline(s: Signals, now: number): number {
  const times = [s.celebrateUntil, s.sleepAfterMs + lastSeen(s)];
  if (s.oneShot) times.push(s.oneShot.until);
  if (s.lastActivity) times.push(s.lastInputAt + s.reactMs);
  return Math.min(Infinity, ...times.filter((t) => t > now));
}

/** Typing frame period: faster typing, faster paws. */
export function typingFrameMs(keysPerSecond: number): number {
  return Math.min(350, Math.max(70, Math.round(350 / Math.max(keysPerSecond, 0.1))));
}

const sum = (ms: number[]) => ms.reduce((a, b) => a + b, 0);

/** Total duration of a one-shot animation. */
export function animDuration(anim: Anim): number {
  return sum(anim.frames.map((f) => f.ms));
}

/** Duration of the frames a looping animation plays only once, before its loop. */
export function introDuration(anim: Anim): number {
  return sum(anim.frames.slice(0, anim.loop ? (anim.loopFrom ?? 0) : 0).map((f) => f.ms));
}

/**
 * Frame index at `elapsed` ms and ms until the next frame change
 * (Infinity once a one-shot animation rests on its last frame).
 * `loopMs` overrides the period of the looping frames.
 */
export function frameAt(anim: Anim, elapsed: number, loopMs?: number): { index: number; nextIn: number } {
  const loopFrom = anim.loop ? (anim.loopFrom ?? 0) : anim.frames.length;
  const durations = anim.frames.map((f, i) => (i >= loopFrom ? (loopMs ?? f.ms) : f.ms));
  const intro = sum(durations.slice(0, loopFrom));
  const total = sum(durations);
  if (!anim.loop && elapsed >= total) return { index: durations.length - 1, nextIn: Infinity };
  let t = anim.loop && elapsed >= intro ? intro + ((elapsed - intro) % (total - intro)) : elapsed;
  for (let i = 0; i < durations.length; i++) {
    if (t < durations[i]) return { index: i, nextIn: durations[i] - t };
    t -= durations[i];
  }
  return { index: durations.length - 1, nextIn: Infinity };
}
