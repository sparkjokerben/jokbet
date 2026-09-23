// Pure animation state selection and frame timing for the pet.

import type { Anim, AnimName } from "../sprites/jokbet";

export type Blocked = "noperm" | "secure";
export type Activity = "typing" | "click";
export type OneShot = "poke" | "hearts" | "soccer" | "wave" | "wake" | "typingEnd";

export interface Signals {
  blocked: Blocked | null;
  dragging: boolean;
  celebrateUntil: number;
  oneShot: { anim: OneShot; until: number } | null;
  lastInputAt: number;
  lastActivity: Activity | null;
  /** How long the last activity keeps the pet reacting; see typingReactMs. */
  reactMs: number;
  sleepAfterMs: number;
}

/** Why the pet cannot see input: no hook, or keys hidden by Secure Input. */
export function blockedBy(s: { permission: string; listening: boolean; secureInput: boolean }): Blocked | null {
  if (s.permission === "denied" || s.permission === "unsupported" || !s.listening) return "noperm";
  if (s.secureInput) return "secure";
  return null;
}

/** How long a reaction lasts after the last input of that kind. Keystrokes
 * are special: see typingReactMs. */
export const REACT_MS: Record<Activity, number> = { typing: 1500, click: 300 };

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

/** Highest priority first: blocked > dragged > celebrate > one-shot > sleep > react > idle. */
export function pickAnim(s: Signals, now: number): AnimName {
  if (s.blocked) return s.blocked;
  if (s.dragging) return "dragged";
  if (now < s.celebrateUntil) return "celebrate";
  if (s.oneShot && now < s.oneShot.until) return s.oneShot.anim;
  if (now - s.lastInputAt >= s.sleepAfterMs) return "sleep";
  if (s.lastActivity && now - s.lastInputAt < s.reactMs) return s.lastActivity;
  return "idle";
}

/** Earliest future time at which pickAnim may change without new signals. */
export function nextDeadline(s: Signals, now: number): number {
  const times = [s.celebrateUntil, s.sleepAfterMs + s.lastInputAt];
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
