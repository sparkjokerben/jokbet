import { lang as defaultLang, t, type Lang } from "./i18n.ts";
import type { HeadCounter, Tick, Totals } from "./types.ts";

export const clicks = (x: Totals) => x.clickLeft + x.clickRight + x.clickMiddle;

/** The single number above the pet's head. */
export function headValue(tick: Tick, h: HeadCounter): number {
  if (h.kind === "rate") return (h.keyboard ? tick.kpm : 0) + (h.mouse ? tick.cpm : 0);
  return (h.keyboard ? tick.today.keys : 0) + (h.mouse ? clicks(tick.today) : 0);
}

const locale = (l: Lang) => (l === "zh" ? "zh-CN" : "en-US");

/** Full digits up to 99,999, then compact (12.3万 / 123.4K). */
export function formatCount(n: number, l: Lang = defaultLang): string {
  const opts: Intl.NumberFormatOptions =
    n < 100_000 ? {} : { notation: "compact", maximumFractionDigits: 1 };
  return new Intl.NumberFormat(locale(l), opts).format(n);
}

export function formatDistance(mm: number, l: Lang = defaultLang): string {
  const m = mm / 1000;
  if (m >= 1000) return t("kilometers", { n: (m / 1000).toFixed(2) }, l);
  return t("meters", { n: m < 10 ? m.toFixed(1) : Math.round(m) }, l);
}
