import { describe, expect, it } from "vitest";
import { celebrationText, formatCount, formatDistance, formatRate, headValue } from "./format";
import { detectLang, messages, t } from "./i18n";
import type { HeadCounter, MilestoneHit, Tick } from "./types";

const tick: Tick = {
  today: { keys: 1000, clickLeft: 50, clickRight: 20, clickMiddle: 5, scrolls: 7, movePx: 0, moveMm: 0 },
  kps: 3.2,
  cps: 0.4,
  activity: null,
};
const head = (over: Partial<HeadCounter>): HeadCounter => ({
  enabled: true,
  kind: "today",
  keyboard: true,
  mouse: false,
  ...over,
});

describe("headValue", () => {
  it("counts keys, clicks or both for today", () => {
    expect(headValue(tick, head({}))).toBe(1000);
    expect(headValue(tick, head({ keyboard: false, mouse: true }))).toBe(75);
    expect(headValue(tick, head({ mouse: true }))).toBe(1075);
  });
  it("sums per-second rates", () => {
    expect(headValue(tick, head({ kind: "rate" }))).toBe(3.2);
    expect(headValue(tick, head({ kind: "rate", mouse: true }))).toBeCloseTo(3.6);
  });
});

describe("formatCount", () => {
  it("keeps full digits below 100k and compacts above", () => {
    expect(formatCount(12345, "en")).toBe("12,345");
    expect(formatCount(123456, "en")).toBe("123.5K");
    expect(formatCount(123456, "zh")).toBe("12.3万");
  });
});

describe("formatRate", () => {
  it("shows one decimal, rounding float noise away", () => {
    expect(formatRate(3.2 + 0.4, "en")).toBe("3.6");
    expect(formatRate(0, "zh")).toBe("0.0");
    expect(formatRate(12, "en")).toBe("12.0");
  });
});

describe("formatDistance", () => {
  it("uses metres then kilometres", () => {
    expect(formatDistance(5300, "en")).toBe("5.3 m");
    expect(formatDistance(123_400, "en")).toBe("123 m");
    expect(formatDistance(1_234_000, "en")).toBe("1.23 km");
    expect(formatDistance(1_234_000, "zh")).toBe("1.23 公里");
  });
});

describe("i18n", () => {
  it("zh and en define the same keys", () => {
    expect(Object.keys(messages.en).sort()).toEqual(Object.keys(messages.zh).sort());
  });
  it("detects Chinese locales", () => {
    expect(detectLang("zh-CN")).toBe("zh");
    expect(detectLang("zh-Hant-TW")).toBe("zh");
    expect(detectLang("en-US")).toBe("en");
    expect(detectLang(undefined)).toBe("en");
  });
  it("fills placeholders", () => {
    expect(t("clickSplit", { l: 1, r: 2, m: 3 }, "en")).toBe("L 1 · R 2 · M 3");
  });
});

describe("celebrationText", () => {
  const hit = (over: Partial<MilestoneHit>): MilestoneHit => ({
    id: "x",
    period: "daily",
    metric: "keys",
    level: 10_000,
    ...over,
  });
  it("names the milestone", () => {
    expect(celebrationText([hit({})], "zh")).toBe("🎉 今天按键 10,000！");
    expect(celebrationText([hit({})], "en")).toBe("🎉 10,000 keys today!");
    expect(celebrationText([hit({ period: "lifetime", level: 1_000_000 })], "zh")).toBe("🎉 累计按键 100万！");
    expect(celebrationText([hit({ metric: "distance", level: 1000 })], "en")).toBe("🎉 1.00 km of mouse travel today!");
  });
  it("leads with lifetime and the highest level, counting the rest", () => {
    const hits = [hit({ level: 1000 }), hit({ level: 5000 }), hit({ period: "lifetime", level: 100_000 })];
    expect(celebrationText(hits, "en")).toBe("🎉 100K keys all time! (+2 more)");
  });
  it("is empty without hits", () => {
    expect(celebrationText([], "en")).toBe("");
  });
});
