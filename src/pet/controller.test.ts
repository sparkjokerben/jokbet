import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";
import { ANIMS, compose, facing } from "../sprites/jokbet";
import { PetController } from "./controller";
import { REACT_MS, TYPING_HOLD_MAX_MS, animDuration, introDuration, type WalkPlan } from "./machine";

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

  it("flinches when the mouse is clicked away from the pet", () => {
    pet.input("click");
    expect(last()).toEqual(compose({ armL: "up", armR: "up" }));
  });

  it("leaves the pet alone when the click landed on it", () => {
    pet.pressed(); // the pointer went down on the pet
    pet.input("click"); // and the engine reports that click a moment later
    expect(last()).not.toEqual(compose({ armL: "up", armR: "up" }));
  });

  it("stops a flinch the engine reported just before the press", () => {
    pet.input("click");
    expect(last()).toEqual(compose({ armL: "up", armR: "up" }));
    pet.pressed();
    expect(last()).not.toEqual(compose({ armL: "up", armR: "up" }));
  });

  it("does not let that flinch cut the pet's own reaction short", () => {
    // Clicking the pet plays one of its reactions; the click the engine sees
    // a moment later must leave it alone.
    pet.oneShot("wave");
    advance(300); // into the second frame of the wave
    const waving = last();
    pet.input("click");
    expect(last()).toEqual(waving);
  });

  it("keeps looking at the cursor whatever the idle animation", () => {
    pet.setGaze([1, 0]);
    expect(last()).toEqual(compose({ gaze: [1, 0] }));
    pet.setIdleChoices(["lookAround"]);
    // Picking it shows it right away: turned left, eyes still on the cursor.
    expect(last()).toEqual(compose({ turn: -1, gaze: [1, 0] }));
  });

  it("plays the idle animation only once in a long while", () => {
    pet.setIdleChoices(["lookAround"]);
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
    // The first keystrokes hold for the base time, then the laptop goes away.
    advance(REACT_MS.typing);
    expect(last()).toEqual(ANIMS.typingEnd.frames[0].rows);
    // Typing again while it is still out sits straight back down at the keys.
    pet.input("typing");
    expect(last()).toEqual(ANIMS.typing.frames[ANIMS.typing.loopFrom ?? 0].rows);
    // And once it is away, the pet is back to breathing.
    const from = frames.length;
    advance(TYPING_HOLD_MAX_MS);
    expect(frames.slice(from)).toContainEqual(compose({}));
    // The hold is about as long as getting the laptop out (see machine.test).
    expect(introDuration(ANIMS.typing) - REACT_MS.typing).toBeLessThanOrEqual(100);
  });

  it("keeps the laptop out longer the longer the spell of typing", () => {
    // Type in bursts for a while: the spell keeps growing.
    for (let t = 0; t < 20_000; t += 500) {
      pet.input("typing");
      advance(500);
    }
    // Pausing for longer than a burst's hold, but not for a long spell's.
    advance(REACT_MS.typing + 200);
    expect(last()).not.toEqual(ANIMS.typingEnd.frames[0].rows);
    // Given a long enough pause, the laptop does go away.
    const from = frames.length;
    advance(TYPING_HOLD_MAX_MS);
    expect(frames.slice(from)).toContainEqual(ANIMS.typingEnd.frames[0].rows);
  });

  it("gets the laptop out again after a long pause", () => {
    pet.input("typing");
    advance(REACT_MS.typing + animDuration(ANIMS.typingEnd) + 10_000);
    pet.input("typing");
    expect(last()).toEqual(ANIMS.typing.frames[0].rows);
  });

  it("keeps awake on scrolls but acts nothing out for them", () => {
    pet.setSleepAfter(60_000);
    advance(50_000);
    pet.input("scroll");
    expect(last()).toEqual(compose({}));
    const asleep = compose(ANIMS.sleep.frames[0].pose);
    let from = frames.length;
    // Past the minute since the last key or click, a minute since the scroll.
    advance(50_000);
    expect(frames.slice(from)).not.toContainEqual(asleep);
    from = frames.length;
    advance(20_000);
    expect(frames.slice(from)).toContainEqual(asleep);
    // A scroll wakes it like anything else.
    pet.input("scroll");
    expect(last()).toEqual(compose(ANIMS.wake.frames[0].pose));
  });

  it("stretches and yawns, then goes back to breathing", () => {
    pet.oneShot("stretch");
    expect(last()).toEqual(compose(ANIMS.stretch.frames[0].pose));
    advance(animDuration(ANIMS.stretch));
    expect(last()).toEqual(compose({}));
  });

  describe("strolls", () => {
    const stride = ANIMS.walk.frames[0].pose!;
    let driver: { start: Mock<(plan: WalkPlan) => boolean>; stop: Mock<() => void> };
    beforeEach(() => {
      driver = { start: vi.fn((_plan: WalkPlan) => true), stop: vi.fn() };
      pet.setWalkDriver(driver);
    });

    it("sets off when a walk is picked, looking where it is going first", () => {
      pet.setIdleChoices(["walk"]);
      // A new choice is shown, not wandered off with: the first walk waits.
      expect(driver.start).not.toHaveBeenCalled();
      advance(IDLE_GAP);
      expect(driver.start).toHaveBeenCalledWith({ speed: 40, legs: [{ dir: 1, distance: 300 }] });
      pet.walkPhase(1, "glance", 1, 88);
      expect(last()).toEqual(compose({ turn: 1, gaze: [1, 0] }));
      pet.walkPhase(1, "walk", 1, 88);
      expect(last()).toEqual(compose({ ...stride, gaze: [1, 0] }));
      advance(88);
      expect(last()).toEqual(compose({ ...ANIMS.walk.frames[1].pose, gaze: [1, 0] }));
    });

    it("turns the stride round to walk left", () => {
      pet.setIdleChoices(["walk"]);
      advance(IDLE_GAP);
      pet.walkPhase(1, "walk", -1, 88);
      // Mirrored whole: turned the other way, the legs and arms swapped round.
      expect(last()).toEqual(compose({ ...facing(stride, -1), gaze: [-1, 0] }));
      expect(facing(stride, -1).turn).toBe(-1);
    });

    it("stops the moment there is input, and ignores what comes after", () => {
      pet.setIdleChoices(["walk"]);
      advance(IDLE_GAP);
      pet.walkPhase(1, "walk", 1, 88);
      pet.input("typing");
      expect(driver.stop).toHaveBeenCalledOnce();
      expect(last()).toEqual(ANIMS.typing.frames[0].rows);
      // A turn reported before the stop got there changes nothing.
      pet.walkPhase(1, "walk", -1, 88);
      expect(last()).toEqual(ANIMS.typing.frames[0].rows);
    });

    it("breathes again when the walk is over", () => {
      pet.setIdleChoices(["walk"]);
      advance(IDLE_GAP);
      pet.walkPhase(1, "walk", 1, 88);
      pet.walkPhase(1, "stop", 1, 88);
      expect(last()).toEqual(compose({}));
      expect(driver.stop).not.toHaveBeenCalled();
    });

    it("does something else when it cannot walk from where it is", async () => {
      driver.start.mockReturnValue(false);
      // With random() at 0.5 the pick of two is the second: the walk.
      pet.setIdleChoices(["lookAround", "walk"]);
      await vi.advanceTimersByTimeAsync(animDuration(ANIMS.lookAround) + IDLE_GAP);
      expect(driver.start).toHaveBeenCalled();
      expect(last()).toEqual(compose({ turn: -1 }));
    });

    it("never walks without a driver", () => {
      pet.setWalkDriver(null);
      pet.setIdleChoices(["walk"]);
      advance(IDLE_GAP * 3);
      expect(driver.start).not.toHaveBeenCalled();
    });
  });

  it("only breathes when no idle animation is chosen", () => {
    pet.setIdleChoices(["soccer"]);
    pet.setIdleChoices([]);
    const from = frames.length;
    advance(IDLE_GAP * 3);
    for (const rows of frames.slice(from)) {
      expect([compose({}), compose({ squash: 1 }), compose({ eyes: "closed" }), compose({ squash: 1, eyes: "closed" })]).toContainEqual(rows);
    }
  });

  it("gets the laptop out again if it was already put away", () => {
    pet.input("typing");
    advance(REACT_MS.typing);
    expect(last()).toEqual(ANIMS.typingEnd.frames[0].rows);
    advance(animDuration(ANIMS.typingEnd) - 1);
    pet.input("typing");
    expect(last()).toEqual(ANIMS.typing.frames[0].rows);
  });
});
