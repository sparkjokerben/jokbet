// What the stats page means by a metric: one label and one number per choice,
// shared by the trend chart, the hours chart and the insights, so all three
// follow the same selector.

import { clicks } from "../../lib/format";
import type { MessageKey } from "../../lib/i18n";
import type { Metric, Totals } from "../../lib/types";

export const METRIC_LABEL: Record<Metric, MessageKey> = {
  keys: "metricKeys",
  clicks: "metricClicks",
  inputs: "metricInputs",
  scrolls: "metricScrolls",
  distance: "metricDistance",
};

/** The metrics the selector offers; "inputs" is only used by the milestones. */
export const METRICS = ["keys", "clicks", "scrolls", "distance"] as const;

const VALUE: Record<Metric, (x: Totals) => number> = {
  keys: (x) => x.keys,
  clicks,
  inputs: (x) => x.keys + clicks(x),
  scrolls: (x) => x.scrolls,
  distance: (x) => x.moveMm,
};

export const valueOf = (x: Totals, m: Metric) => VALUE[m](x);
