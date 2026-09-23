import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ANIMS, compose } from "../sprites/jokbet";
import { PetController } from "./controller";
import { REACT_MS, animDuration, introDuration } from "./machine";

/** The gap between idle animations with Math.random() at 0.5. */
const IDLE_GAP = 40_000;

describe("PetController", () => {
  let frames: string[][] = [];
  let pet: PetController;
  const last = () => frames[frames.length - 1];
  // Fake timers move Date.now() along as each timer fires.
  const advance = (ms: number) => vi.advanceTimersByTime(ms);

  beforeEach(() => {
    // Fixed blink and idle-animation gaps.
    vi.spyOn(Math, "random").mockReturnValue(0.5);
    vi.useFakeTimers();
    vi.setSystemTime(0);
    frames = [];
    pet = new PetController((rows) => frames.push(rows), () => Date.now());
  });
  afterEach(() => {
    pet.destroy();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("keeps looking at the cursor whatever the idle animation", () => {
    pet.setGaze([1, 0]);
    expect(last()).toEqual(compose({ gaze: [1, 0] }));
    pet.setIdleAnim("lookAround");
    // Picking it shows it right away: turned left, eyes still on the cursor.
    expect(last()).toEqual(compose({ turn: -1, gaze: [1, 0] }));
  });

  it("plays the idle animation only once in a long while", () => {
    pet.setIdleAnim("lookAround");
    advance(animDuration(ANIMS.lookAround));
    const turned = compose({ turn: -1 });
    const from = frames.length;
    advance(IDLE_GAP - 100);
    expect(frames.slice(from)).not.toContainEqual(turned);
    advance(200);
    expect(frames.slice(from)).toContainEqual(turned);
  });

  it("puts the laptop away when typing stops, and resumes without getting it out again", () => {
    pet.input("typing");
    expect(last()).toEqual(ANIMS.typing.frames[0].rows);
    advance(REACT_MS.typing);
    expect(last()).toEqual(ANIMS.typingEnd.frames[0].rows);
    pet.input("typing");
    expect(last()).toEqual(ANIMS.typing.frames[ANIMS.typing.loopFrom ?? 0].rows);
    advance(REACT_MS.typing + animDuration(ANIMS.typingEnd));
    // The laptop is away and the pet is back to breathing.
    expect(last()).toEqual(compose({}));
    expect(introDuration(ANIMS.typing)).toBeLessThan(REACT_MS.typing);
  });
});
