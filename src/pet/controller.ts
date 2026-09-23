// Drives the pet's animation: tracks signals, applies blink/gaze/pause
// overlays and schedules the next frame with plain timers (no rAF loop).

import { ANIMS, compose, frameRows, type AnimName, type Effect, type Frame, type Pose } from "../sprites/jokbet";
import {
  animDuration,
  frameAt,
  introDuration,
  nextDeadline,
  pickAnim,
  typingFrameMs,
  type Activity,
  type Blocked,
  type OneShot,
  type Signals,
} from "./machine";

const BLINK_MS = 140;
const blinkGap = () => 3000 + Math.random() * 3000;
/** Idle time before the idle animation plays, and again between plays. */
const idleShowGap = () => 20_000 + Math.random() * 40_000;

export class PetController {
  private signals: Signals;
  private anim: AnimName = "idle";
  /** What the idle state plays now and then between breaths: soccer or looking around. */
  private idleAnim: AnimName = "idle";
  private idleShowAt = Infinity;
  private idleShowUntil = 0;
  private animStart = 0;
  private gaze: readonly [number, number] = [0, 0];
  private blinkAt: number;
  private paused = false;
  private keysPerSecond = 0;
  /** Typing resumed while the laptop was being put away: skip getting it out again. */
  private resumeTyping = false;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private lastKey = "";

  constructor(
    private readonly onRender: (rows: string[]) => void,
    private readonly now: () => number = () => performance.now(),
  ) {
    const t = this.now();
    this.signals = {
      blocked: null,
      dragging: false,
      celebrateUntil: 0,
      oneShot: null,
      lastInputAt: t,
      lastActivity: null,
      sleepAfterMs: 5 * 60_000,
    };
    this.animStart = t;
    this.blinkAt = t + blinkGap();
    this.update();
  }

  input(activity: Activity) {
    const t = this.now();
    if (this.anim === "sleep") this.oneShot("wake");
    if (activity === "typing" && this.signals.oneShot?.anim === "typingEnd") {
      this.signals.oneShot = null;
      this.resumeTyping = true;
    }
    if (activity === "click") this.animStart = t;
    this.signals.lastInputAt = t;
    this.signals.lastActivity = activity;
    this.update();
  }

  oneShot(anim: OneShot) {
    const t = this.now();
    this.signals.oneShot = { anim, until: t + animDuration(ANIMS[anim]) };
    this.animStart = t;
    this.update();
  }

  celebrate(ms = 3000) {
    this.signals.celebrateUntil = this.now() + ms;
    this.update();
  }

  setDragging(dragging: boolean) {
    this.signals.dragging = dragging;
    this.update();
  }

  setBlocked(blocked: Blocked | null) {
    this.signals.blocked = blocked;
    this.update();
  }

  setGaze(gaze: readonly [number, number]) {
    this.gaze = gaze;
    this.update();
  }

  setPaused(paused: boolean) {
    this.paused = paused;
    this.update();
  }

  setKeysPerSecond(kps: number) {
    this.keysPerSecond = kps;
  }

  /** Picks the idle animation, and shows it once right away if idling. */
  setIdleAnim(anim: AnimName) {
    if (anim === this.idleAnim) return;
    this.idleAnim = anim;
    this.idleShowUntil = 0;
    this.idleShowAt = this.now();
    this.update();
  }

  setSleepAfter(ms: number) {
    this.signals.sleepAfterMs = ms;
    this.update();
  }

  destroy() {
    clearTimeout(this.timer);
  }

  private update() {
    clearTimeout(this.timer);
    const t = this.now();
    let next = pickAnim(this.signals, t);
    if (this.anim === "typing" && next === "idle") {
      // Put the laptop away before idling.
      this.signals.oneShot = { anim: "typingEnd", until: t + animDuration(ANIMS.typingEnd) };
      next = "typingEnd";
    }
    if (next !== this.anim) {
      if (this.anim === "idle") this.idleShowUntil = 0;
      if (next === "idle" && this.idleShowAt < t) this.idleShowAt = t + idleShowGap();
      this.anim = next;
      this.animStart = next === "typing" && this.resumeTyping ? t - introDuration(ANIMS.typing) : t;
    }
    this.resumeTyping = false;

    // Idling is breathing, with the idle animation once in a long while.
    let name = this.anim;
    const showing = this.anim === "idle" && this.idleAnim !== "idle";
    if (showing && t >= this.idleShowAt) {
      this.idleShowUntil = t + animDuration(ANIMS[this.idleAnim]);
      this.idleShowAt = this.idleShowUntil + idleShowGap();
      this.animStart = t;
    }
    if (showing && t < this.idleShowUntil) name = this.idleAnim;
    const anim = ANIMS[name];
    const { index, nextIn } = frameAt(
      anim,
      t - this.animStart,
      this.anim === "typing" ? typingFrameMs(this.keysPerSecond) : undefined,
    );
    const frame: Frame = anim.frames[index];
    // Frames recovered from the reference videos are pixels; the rest are poses
    // the overlays below can still bend.
    const pose: Pose | null = frame.rows ? null : { ...frame.pose };

    let wakeAt = Math.min(t + nextIn, nextDeadline(this.signals, t));
    if (showing) wakeAt = Math.min(wakeAt, t < this.idleShowUntil ? this.idleShowUntil : this.idleShowAt);
    if (pose && (this.anim === "idle" || this.anim === "scroll")) {
      // Whatever the idle animation does, the eyes stay on the cursor.
      if (this.anim === "idle") pose.gaze = this.gaze;
      if (t >= this.blinkAt + BLINK_MS) this.blinkAt = t + blinkGap();
      if (t >= this.blinkAt) pose.eyes = "closed";
      wakeAt = Math.min(wakeAt, t >= this.blinkAt ? this.blinkAt + BLINK_MS : this.blinkAt);
    }
    if (pose && this.paused && !this.signals.blocked) {
      pose.fx = [...(pose.fx ?? []), "pause" as Effect];
    }

    const rows = pose ? compose(pose) : frameRows(frame);
    const key = rows.join("\n");
    if (key !== this.lastKey) {
      this.lastKey = key;
      this.onRender([...rows]);
    }
    if (Number.isFinite(wakeAt)) {
      this.timer = setTimeout(() => this.update(), Math.max(0, wakeAt - t));
    }
  }
}
