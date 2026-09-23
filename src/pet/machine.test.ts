import { describe, expect, it } from "vitest";
import { ANIMS } from "../sprites/jokbet";
import {
  REACT_MS,
  TYPING_HOLD_MAX_MS,
  blockedBy,
  frameAt,
  introDuration,
  nextDeadline,
  pickAnim,
  typingFrameMs,
  typingReactMs,
  type Signals,
} from "./machine";

const base = (over: Partial<Signals> = {}): Signals => ({
  blocked: null,
  dragging: false,
  celebrateUntil: 0,
  oneShot: null,
  lastInputAt: 0,
  lastActivity: null,
  reactMs: REACT_MS.typing,
  sleepAfterMs: 60_000,
  ...over,
});

describe("pickAnim priorities", () => {
  it("blocked beats everything", () => {
    const s = base({ blocked: "noperm", dragging: true, celebrateUntil: 10_000 });
    expect(pickAnim(s, 100)).toBe("noperm");
  });
  it("dragged beats celebrate and one-shots", () => {
    const s = base({ dragging: true, celebrateUntil: 10_000, oneShot: { anim: "poke", until: 10_000 } });
    expect(pickAnim(s, 100)).toBe("dragged");
  });
  it("celebrate beats one-shots", () => {
    const s = base({ celebrateUntil: 10_000, oneShot: { anim: "hearts", until: 10_000 } });
    expect(pickAnim(s, 100)).toBe("celebrate");
  });
  it("one-shot beats sleep", () => {
    const s = base({ oneShot: { anim: "wake", until: 70_500 } });
    expect(pickAnim(s, 70_000)).toBe("wake");
  });
  it("sleeps after the idle timeout", () => {
    expect(pickAnim(base(), 59_999)).toBe("idle");
    expect(pickAnim(base(), 60_000)).toBe("sleep");
  });
  it("reacts briefly to input, then idles", () => {
    const s = base({ lastInputAt: 1000, lastActivity: "typing" });
    expect(pickAnim(s, 1999)).toBe("typing");
    expect(pickAnim(s, 2000)).toBe("idle");
  });
});

describe("typingReactMs", () => {
  it("grows with the spell of typing and stops growing", () => {
    expect(typingReactMs(0)).toBe(REACT_MS.typing);
    expect(typingReactMs(1000)).toBeGreaterThan(REACT_MS.typing);
    expect(typingReactMs(10_000)).toBeGreaterThan(typingReactMs(1000));
    expect(typingReactMs(10 * 60_000)).toBe(TYPING_HOLD_MAX_MS);
    // Growth is sub-linear: ten times the typing is well under ten times the hold.
    expect(typingReactMs(10_000) - REACT_MS.typing).toBeLessThan(
      10 * (typingReactMs(1000) - REACT_MS.typing),
    );
  });
});

describe("nextDeadline", () => {
  it("returns the earliest future transition", () => {
    const s = base({ lastInputAt: 1000, lastActivity: "typing", celebrateUntil: 5000 });
    expect(nextDeadline(s, 1200)).toBe(2000);
    expect(nextDeadline(s, 2600)).toBe(5000);
    expect(nextDeadline(s, 6000)).toBe(61_000);
  });
});

describe("frameAt", () => {
  it("loops looping animations", () => {
    expect(frameAt(ANIMS.idle, 0)).toEqual({ index: 0, nextIn: 1400 });
    expect(frameAt(ANIMS.idle, 1500)).toEqual({ index: 1, nextIn: 400 });
    expect(frameAt(ANIMS.idle, 1900)).toEqual({ index: 0, nextIn: 1400 });
  });
  it("rests one-shots on the last frame", () => {
    expect(frameAt(ANIMS.poke, 100)).toEqual({ index: 0, nextIn: 100 });
    expect(frameAt(ANIMS.poke, 10_000)).toEqual({ index: 2, nextIn: Infinity });
  });
  it("plays the intro once, then loops at the typing override", () => {
    const intro = introDuration(ANIMS.typing);
    const loop = ANIMS.typing.frames.length - (ANIMS.typing.loopFrom ?? 0);
    expect(intro).toBe(1036);
    expect(loop).toBe(3);
    expect(frameAt(ANIMS.typing, 100, 70)).toEqual({ index: 1, nextIn: 34 });
    expect(frameAt(ANIMS.typing, intro, 70)).toEqual({ index: ANIMS.typing.loopFrom, nextIn: 70 });
    expect(frameAt(ANIMS.typing, intro + 100, 70).index).toBe((ANIMS.typing.loopFrom ?? 0) + 1);
    // The base hold is about as long as getting the laptop out takes: any
    // shorter and the pet would start putting it away before it had it out.
    expect(intro - REACT_MS.typing).toBeLessThanOrEqual(100);
    expect(intro - REACT_MS.typing).toBeGreaterThanOrEqual(-500);
  });
});

describe("typingFrameMs", () => {
  it("speeds up with keys per second within bounds", () => {
    expect(typingFrameMs(0)).toBe(350);
    expect(typingFrameMs(2.5)).toBe(140);
    expect(typingFrameMs(20)).toBe(70);
  });
});

describe("blockedBy", () => {
  it("missing permission or a dead hook means noperm", () => {
    expect(blockedBy({ permission: "denied", listening: false, secureInput: false })).toBe("noperm");
    expect(blockedBy({ permission: "granted", listening: false, secureInput: true })).toBe("noperm");
  });
  it("secure input blindfolds the pet", () => {
    expect(blockedBy({ permission: "granted", listening: true, secureInput: true })).toBe("secure");
  });
  it("otherwise nothing blocks", () => {
    expect(blockedBy({ permission: "notRequired", listening: true, secureInput: false })).toBeNull();
  });
});
