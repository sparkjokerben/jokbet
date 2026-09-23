import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ANIMS, compose } from "../sprites/jokbet";
import { PetController } from "./controller";
import { REACT_MS, animDuration, introDuration } from "./machine";

describe("PetController", () => {
  let frames: string[][] = [];
  let pet: PetController;
  const last = () => frames[frames.length - 1];
  // Fake timers move Date.now() along as each timer fires.
  const advance = (ms: number) => vi.advanceTimersByTime(ms);

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(0);
    frames = [];
    pet = new PetController((rows) => frames.push(rows), () => Date.now());
  });
  afterEach(() => {
    pet.destroy();
    vi.useRealTimers();
  });

  it("keeps looking at the cursor whatever the idle animation", () => {
    pet.setIdleAnim("lookAround");
    pet.setGaze([1, 0]);
    expect(last()).toEqual(compose({ gaze: [1, 0] }));
    advance(1900); // turns left, eyes still on the cursor
    expect(last()).toEqual(compose({ turn: -1, gaze: [1, 0] }));
  });

  it("puts the laptop away when typing stops, and resumes without getting it out again", () => {
    pet.input("typing");
    expect(last()).toEqual(compose(ANIMS.typing.frames[0].pose));
    advance(REACT_MS.typing);
    expect(last()).toEqual(compose(ANIMS.typingEnd.frames[0].pose));
    pet.input("typing");
    expect(last()).toEqual(compose(ANIMS.typing.frames[3].pose));
    advance(REACT_MS.typing + animDuration(ANIMS.typingEnd));
    expect(last()).toEqual(compose({}));
    expect(introDuration(ANIMS.typing)).toBeLessThan(REACT_MS.typing);
  });
});
