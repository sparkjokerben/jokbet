import { describe, expect, it } from "vitest";
import { cardBody, inkSide } from "./ink";

describe("the ink", () => {
  it("changes hands where the panel is plainly one way", () => {
    // A pale desktop, all the way down to where the band starts.
    expect(inkSide(0, false)).toBe(false);
    expect(inkSide(0.4, false)).toBe(false);
    // And a dark one, all the way up to it.
    expect(inkSide(1, false)).toBe(true);
    expect(inkSide(0.6, false)).toBe(true);
  });

  it("keeps the ink it has across the middle of the band", () => {
    // Which is exactly where a wallpaper's luma swings as the pet is carried
    // over it: changing hands over each step of that swing is the panel
    // flickering, which is what a drag across a photograph looked like.
    for (const tone of [0.45, 0.5, 0.55]) {
      expect(inkSide(tone, true)).toBe(true);
      expect(inkSide(tone, false)).toBe(false);
    }
  });
});

describe("the card's own colour", () => {
  it("carries none where the desktop is plainly one way", () => {
    expect(cardBody(0)).toBe(0);
    expect(cardBody(1)).toBe(0);
    expect(cardBody(0.2)).toBe(0);
    expect(cardBody(0.8)).toBe(0);
  });

  it("carries the most of it where the desktop is too mixed to read against", () => {
    const middle = cardBody(0.5);
    expect(middle).toBeCloseTo(0.3);
    // And it comes on a step at a time on the way in, never as a step of its
    // own: the card is what the panel follows the desktop with.
    expect(cardBody(0.3)).toBeGreaterThan(0);
    expect(cardBody(0.4)).toBeGreaterThan(cardBody(0.3));
    expect(middle).toBeGreaterThan(cardBody(0.4));
  });
});
