// Drives the pet's animation: tracks signals, applies blink/gaze/pause
// overlays and schedules the next frame with plain timers (no rAF loop).

import { ANIMS, compose, type AnimName, type Effect, type Pose } from "../sprites/clawd";
import {
  animDuration,
  frameAt,
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

export class PetController {
  private signals: Signals;
  private anim: AnimName = "idle";
  private animStart = 0;
  private gaze: readonly [number, number] = [0, 0];
  private blinkAt: number;
  private paused = false;
  private kpm = 0;
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

  setKpm(kpm: number) {
    this.kpm = kpm;
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
    const next = pickAnim(this.signals, t);
    if (next !== this.anim) {
      this.anim = next;
      this.animStart = t;
    }
    const anim = ANIMS[this.anim];
    const { index, nextIn } = frameAt(
      anim,
      t - this.animStart,
      this.anim === "typing" ? typingFrameMs(this.kpm) : undefined,
    );
    const pose: Pose = { ...anim.frames[index].pose };

    let wakeAt = Math.min(t + nextIn, nextDeadline(this.signals, t));
    if (this.anim === "idle" || this.anim === "scroll") {
      if (this.anim === "idle") pose.gaze ??= this.gaze;
      if (t >= this.blinkAt + BLINK_MS) this.blinkAt = t + blinkGap();
      if (t >= this.blinkAt) pose.eyes = "closed";
      wakeAt = Math.min(wakeAt, t >= this.blinkAt ? this.blinkAt + BLINK_MS : this.blinkAt);
    }
    if (this.paused && !this.signals.blocked) {
      pose.fx = [...(pose.fx ?? []), "pause" as Effect];
    }

    const key = JSON.stringify(pose);
    if (key !== this.lastKey) {
      this.lastKey = key;
      this.onRender(compose(pose));
    }
    if (Number.isFinite(wakeAt)) {
      this.timer = setTimeout(() => this.update(), Math.max(0, wakeAt - t));
    }
  }
}
