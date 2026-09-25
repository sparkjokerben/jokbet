import { describe, expect, it } from "vitest";
import { columnAt, columnPath, heatBin, labelIndexes, niceMax } from "./chart";

describe("niceMax", () => {
  it("rounds up to the next clean step", () => {
    expect(niceMax(0)).toBe(1);
    expect(niceMax(7)).toBe(8);
    expect(niceMax(12)).toBe(15);
    expect(niceMax(12_500)).toBe(15_000);
    expect(niceMax(3400)).toBe(4000);
    expect(niceMax(5000)).toBe(5000);
    expect(niceMax(0.3)).toBeCloseTo(0.3);
  });
});

describe("columnPath", () => {
  it("rounds only the top corners", () => {
    expect(columnPath(0, 10, 20, 30)).toBe("M0 40V14Q0 10 4 10H16Q20 10 20 14V40Z");
  });
  it("shrinks the radius for tiny bars and skips empty ones", () => {
    expect(columnPath(0, 38, 20, 2)).toBe("M0 40V40Q0 38 2 38H18Q20 38 20 40V40Z");
    expect(columnPath(0, 40, 20, 0)).toBe("");
  });
});

describe("heatBin", () => {
  it("maps zero to -1 and the max to the darkest bin", () => {
    expect(heatBin(0, 100)).toBe(-1);
    expect(heatBin(100, 100)).toBe(7);
    expect(heatBin(1, 100)).toBe(0);
    expect(heatBin(25, 100)).toBe(4);
  });
});

describe("labelIndexes", () => {
  it("spreads labels and keeps both ends", () => {
    expect(labelIndexes(3)).toEqual([0, 1, 2]);
    expect(labelIndexes(30)).toEqual([0, 7, 15, 22, 29]);
    expect(labelIndexes(0)).toEqual([]);
  });
});

describe("columnAt", () => {
  // Bands 40 wide starting 48 in, as they are on a hit rectangle that starts
  // after the y-axis: the columns run 48..88, 88..128 and so on.
  it("counts from the left edge it is given", () => {
    expect(columnAt(48, 48, 40, 24)).toBe(0);
    expect(columnAt(87, 48, 40, 24)).toBe(0);
    expect(columnAt(88, 48, 40, 24)).toBe(1);
    expect(columnAt(1000, 48, 40, 24)).toBe(23);
  });
  it("has nothing to report outside the area", () => {
    expect(columnAt(47, 48, 40, 24)).toBeNull();
    expect(columnAt(1008, 48, 40, 24)).toBeNull();
    expect(columnAt(48, 48, 0, 24)).toBeNull();
    expect(columnAt(48, 48, 40, 0)).toBeNull();
  });
});
