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
  pickIdle,
  typingFrameMs,
  typingReactMs,
  walkFrameMs,
  walkPlan,
  type Signals,
} from "./machine";

const base = (over: Partial<Signals> = {}): Signals => ({
  blocked: null,
  dragging: false,
  celebrateUntil: 0,
  oneShot: null,
  lastInputAt: 0,
  lastScrollAt: 0,
  lastActivity: null,
  reactMs: REACT_MS.typing,
  sleepAfterMs: 60_000,
  walk: null,
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
  it("stays awake while scrolling, without reacting to it", () => {
    const s = base({ lastInputAt: 1000, lastActivity: "typing", lastScrollAt: 50_000 });
    expect(pickAnim(s, 60_000)).toBe("idle");
    expect(pickAnim(s, 110_000)).toBe("sleep");
    expect(nextDeadline(s, 60_000)).toBe(110_000);
  });
});

describe("walking", () => {
  const walk = { id: 1, dir: 1 as const, glance: false, frameMs: 90 };
  it("comes after everything but breathing", () => {
    expect(pickAnim(base({ walk }), 100)).toBe("walk");
    expect(pickAnim(base({ walk, lastInputAt: 100, lastActivity: "typing" }), 200)).toBe("typing");
    expect(pickAnim(base({ walk, oneShot: { anim: "wave", until: 1000 } }), 100)).toBe("wave");
    expect(pickAnim(base({ walk }), 60_000)).toBe("sleep");
    expect(pickAnim(base({ walk, dragging: true }), 100)).toBe("dragged");
  });
  it("steps a cell of ground a frame", () => {
    expect(walkFrameMs(40, 3.5)).toBe(88);
    expect(walkFrameMs(30, 7)).toBe(233);
    expect(walkFrameMs(50, 1)).toBe(60);
    expect(walkFrameMs(1, 7)).toBe(250);
  });
  it("plans a stroll either way, sometimes turning back part way", () => {
    // Low rolls: left, slow and short, then a change of mind back to the right.
    expect(walkPlan(() => 0)).toEqual({
      speed: 30,
      legs: [
        { dir: -1, distance: 100 },
        { dir: 1, distance: 60 },
      ],
    });
    // High rolls: right, as far as it goes, and no turning back.
    const far = walkPlan(() => 0.999);
    expect(far.legs).toHaveLength(1);
    expect(far.legs[0].dir).toBe(1);
    expect(far.speed).toBeLessThanOrEqual(50);
    expect(far.legs[0].distance).toBeLessThanOrEqual(500);
  });
  it("picks among the chosen idle animations", () => {
    expect(pickIdle([], () => 0.5)).toBeNull();
    expect(pickIdle(["soccer", "lookAround", "walk"], () => 0)).toBe("soccer");
    expect(pickIdle(["soccer", "lookAround", "walk"], () => 0.5)).toBe("lookAround");
    expect(pickIdle(["soccer", "lookAround", "walk"], () => 0.99)).toBe("walk");
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
    expect(blockedBy({ permission: "denied", listening: false })).toBe("noperm");
    expect(blockedBy({ permission: "granted", listening: false })).toBe("noperm");
  });
  it("otherwise nothing blocks", () => {
    expect(blockedBy({ permission: "granted", listening: true })).toBeNull();
    expect(blockedBy({ permission: "notRequired", listening: true })).toBeNull();
  });
});
