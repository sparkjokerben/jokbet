import { describe, expect, it } from "vitest";
import { ANIMS } from "../sprites/jokbet";
import { blockedBy, frameAt, introDuration, nextDeadline, pickAnim, typingFrameMs, type Signals } from "./machine";

const base = (over: Partial<Signals> = {}): Signals => ({
  blocked: null,
  dragging: false,
  celebrateUntil: 0,
  oneShot: null,
  lastInputAt: 0,
  lastActivity: null,
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
    expect(pickAnim(s, 2499)).toBe("typing");
    expect(pickAnim(s, 2500)).toBe("idle");
    const c = base({ lastInputAt: 1000, lastActivity: "click" });
    expect(pickAnim(c, 1299)).toBe("click");
    expect(pickAnim(c, 1300)).toBe("idle");
  });
});

describe("nextDeadline", () => {
  it("returns the earliest future transition", () => {
    const s = base({ lastInputAt: 1000, lastActivity: "typing", celebrateUntil: 5000 });
    expect(nextDeadline(s, 1200)).toBe(2500);
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
    expect(frameAt(ANIMS.poke, 10_000)).toEqual({ index: 1, nextIn: Infinity });
  });
  it("plays the intro once, then loops at the typing override", () => {
    const intro = introDuration(ANIMS.typing);
    expect(intro).toBe(460);
    expect(frameAt(ANIMS.typing, 100, 70)).toEqual({ index: 0, nextIn: 60 });
    expect(frameAt(ANIMS.typing, intro, 70)).toEqual({ index: 3, nextIn: 70 });
    expect(frameAt(ANIMS.typing, intro + 100, 70)).toEqual({ index: 4, nextIn: 40 });
    expect(frameAt(ANIMS.typing, intro + 140, 70)).toEqual({ index: 3, nextIn: 70 });
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
