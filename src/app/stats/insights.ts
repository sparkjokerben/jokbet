// The all-time insights: what the selected range cannot answer, worked out
// from the sparse history the stats window carries beside it.

import type { DayStat, Metric } from "../../lib/types";
import { valueOf } from "./metric";

/** Days since the epoch, so dates compare without a timezone in the way. */
function dayNumber(date: string): number {
  const [y, m, d] = date.split("-").map(Number);
  return Math.floor(Date.UTC(y, m - 1, d) / 86_400_000);
}

/**
 * The day with the most of `metric`, or null when there is none. A day the
 * clock put in the future is ignored rather than winning for good.
 */
export function busiestDay(history: DayStat[], metric: Metric, today: string): DayStat | null {
  const last = dayNumber(today);
  let best: DayStat | null = null;
  let bestValue = 0;
  for (const day of history) {
    const n = dayNumber(day.date);
    if (!Number.isFinite(n) || n > last) continue;
    const value = valueOf(day, metric);
    if (value > bestValue) {
      best = day;
      bestValue = value;
    }
  }
  return best;
}

export interface Streaks {
  /** Days in a row up to today. A day that has not started yet is not a break. */
  current: number;
  longest: number;
}

/**
 * Runs of consecutive days with anything in them. The history has no rows for
 * days the app was closed, so a run only ever joins days it actually saw.
 * `todayValue` comes from the selected range, which always ends today even when
 * nothing has been counted yet.
 */
export function streaks(
  history: DayStat[],
  metric: Metric,
  today: string,
  todayValue: number,
): Streaks {
  const last = dayNumber(today);
  const days = new Set<number>();
  for (const day of history) {
    const n = dayNumber(day.date);
    if (Number.isFinite(n) && n <= last && valueOf(day, metric) > 0) days.add(n);
  }
  if (todayValue > 0) days.add(last);

  let current = 0;
  for (let n = days.has(last) ? last : last - 1; days.has(n); n--) current++;

  let longest = 0;
  for (const n of days) {
    if (days.has(n - 1)) continue; // only count a run from its first day
    let run = 0;
    for (let d = n; days.has(d); d++) run++;
    if (run > longest) longest = run;
  }

  return { current, longest };
}

export interface Week {
  /** The last seven days, today among them. */
  thisWeek: number;
  /** The seven days before those. */
  lastWeek: number;
}

/** The week so far against the one before it, both seven whole days. */
export function weekOverWeek(history: DayStat[], metric: Metric, today: string): Week {
  const last = dayNumber(today);
  const week = { thisWeek: 0, lastWeek: 0 };
  for (const day of history) {
    const n = dayNumber(day.date);
    if (!Number.isFinite(n) || n > last || n <= last - 14) continue;
    week[n > last - 7 ? "thisWeek" : "lastWeek"] += valueOf(day, metric);
  }
  return week;
}
