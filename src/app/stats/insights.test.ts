import { describe, expect, it } from "vitest";
import type { DayStat } from "../../lib/types";
import { busiestDay, streaks, weekOverWeek } from "./insights";

const TODAY = "2026-09-25";

function day(date: string, keys: number, moveMm = 0): DayStat {
  return { date, keys, clickLeft: 0, clickRight: 0, clickMiddle: 0, scrolls: 0, movePx: 0, moveMm };
}

const days = (...dates: string[]) => dates.map((d) => day(d, 10));

describe("busiestDay", () => {
  it("picks the day with the most of the chosen metric", () => {
    const history = [day("2026-09-23", 5, 900), day("2026-09-24", 40, 0)];
    expect(busiestDay(history, "keys", TODAY)?.date).toBe("2026-09-24");
    expect(busiestDay(history, "distance", TODAY)?.date).toBe("2026-09-23");
  });

  it("has nothing to show before anything is counted", () => {
    expect(busiestDay([], "keys", TODAY)).toBeNull();
    expect(busiestDay([day("2026-09-24", 0)], "keys", TODAY)).toBeNull();
  });

  it("ignores a day the clock put in the future", () => {
    const history = [day("2026-09-24", 10), day("2027-01-01", 9999)];
    expect(busiestDay(history, "keys", TODAY)?.date).toBe("2026-09-24");
  });

  it("ignores a row whose date does not parse", () => {
    const history = [{ ...day("nonsense", 9999) }, day("2026-09-24", 10)];
    expect(busiestDay(history, "keys", TODAY)?.date).toBe("2026-09-24");
  });
});

describe("streaks", () => {
  it("counts the run of days up to today", () => {
    expect(streaks(days("2026-09-22", "2026-09-23", "2026-09-24"), "keys", TODAY, 10)).toEqual({
      current: 4,
      longest: 4,
    });
  });

  it("does not break on a day that has not been counted yet", () => {
    // Today is past the end of the history and the range says it is still zero.
    expect(streaks(days("2026-09-22", "2026-09-23", "2026-09-24"), "keys", TODAY, 0).current).toBe(3);
  });

  it("reports no current streak when neither today nor yesterday has anything", () => {
    expect(streaks(days("2026-09-22", "2026-09-23"), "keys", TODAY, 0).current).toBe(0);
  });

  it("keeps the longest run after it is over", () => {
    const history = days("2026-09-10", "2026-09-11", "2026-09-12", "2026-09-13", "2026-09-24");
    expect(streaks(history, "keys", TODAY, 0)).toEqual({ current: 1, longest: 4 });
  });

  it("reads a day through the metric that was chosen", () => {
    const clicked: DayStat = { ...day("2026-09-24", 0), clickLeft: 7 };
    const history = [clicked, day("2026-09-25", 3)];
    expect(streaks(history, "keys", TODAY, 3).current).toBe(1);
    expect(streaks(history, "clicks", TODAY, 7).current).toBe(2);
  });

  it("ignores a day the clock put in the future", () => {
    expect(streaks([...days("2026-09-24"), day("2026-09-26", 10)], "keys", TODAY, 0).current).toBe(1);
  });
});

describe("weekOverWeek", () => {
  it("splits the last fourteen days into two weeks", () => {
    const history = [
      day("2026-09-11", 500), // the day before last week: out
      day("2026-09-12", 1), // last week, oldest
      day("2026-09-18", 2), // last week, newest
      day("2026-09-19", 4), // this week, oldest
      day("2026-09-25", 8), // today
    ];
    expect(weekOverWeek(history, "keys", TODAY)).toEqual({ thisWeek: 12, lastWeek: 3 });
  });

  it("counts a week that is only partly there", () => {
    expect(weekOverWeek([day("2026-09-25", 8)], "keys", TODAY)).toEqual({ thisWeek: 8, lastWeek: 0 });
  });

  it("ignores a day the clock put in the future", () => {
    const history = [day("2026-09-25", 8), day("2026-09-26", 1000)];
    expect(weekOverWeek(history, "keys", TODAY)).toEqual({ thisWeek: 8, lastWeek: 0 });
  });
});
