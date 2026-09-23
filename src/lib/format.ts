import { lang as defaultLang, t, type Lang } from "./i18n.ts";
import type { HeadCounter, Metric, MilestoneHit, Tick, Totals } from "./types.ts";

export const clicks = (x: Totals) => x.clickLeft + x.clickRight + x.clickMiddle;

/** The single number above the pet's head. */
export function headValue(tick: Tick, h: HeadCounter): number {
  if (h.kind === "rate") return (h.keyboard ? tick.kps : 0) + (h.mouse ? tick.cps : 0);
  return (h.keyboard ? tick.today.keys : 0) + (h.mouse ? clicks(tick.today) : 0);
}

const locale = (l: Lang) => (l === "zh" ? "zh-CN" : "en-US");

/** Full digits up to 99,999, then compact (12.3万 / 123.4K). */
export function formatCount(n: number, l: Lang = defaultLang): string {
  const opts: Intl.NumberFormatOptions =
    n < 100_000 ? {} : { notation: "compact", maximumFractionDigits: 1 };
  return new Intl.NumberFormat(locale(l), opts).format(n);
}

/** A per-second rate, always with one decimal. */
export function formatRate(n: number, l: Lang = defaultLang): string {
  return new Intl.NumberFormat(locale(l), { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(n);
}

export function formatDistance(mm: number, l: Lang = defaultLang): string {
  const m = mm / 1000;
  if (m >= 1000) return t("kilometers", { n: (m / 1000).toFixed(2) }, l);
  return t("meters", { n: m < 10 ? m.toFixed(1) : Math.round(m) }, l);
}

const METRIC_WORD: Record<Metric, Parameters<typeof t>[0]> = {
  keys: "metricKeys",
  clicks: "metricClicks",
  scrolls: "metricScrolls",
  distance: "metricDistanceVerb",
};

/** Banner text for a celebration: the most notable hit, plus how many more. */
export function celebrationText(hits: MilestoneHit[], l: Lang = defaultLang): string {
  if (!hits.length) return "";
  const lifetime = hits.filter((h) => h.period === "lifetime");
  const pool = lifetime.length ? lifetime : hits;
  const top = pool.reduce((a, b) => (b.level >= a.level ? b : a));
  const n = top.metric === "distance" ? formatDistance(top.level * 1000, l) : formatCount(top.level, l);
  const metric = t(METRIC_WORD[top.metric], {}, l);
  const text = t(top.period === "daily" ? "celebrateDaily" : "celebrateLifetime", {
    n,
    metric: l === "en" ? metric.toLowerCase() : metric,
  }, l);
  return `🎉 ${text}${hits.length > 1 ? t("celebrateMore", { n: hits.length - 1 }, l) : ""}`;
}
