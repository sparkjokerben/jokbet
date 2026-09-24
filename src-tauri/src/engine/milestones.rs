//! Milestones: which thresholds were crossed, and when to celebrate them.
//! Pure; the caller supplies dates, totals and the clock.

use super::aggregator::Totals;
use crate::settings::{CustomMilestone, HeadCounter, Metric, Period};
use serde::Serialize;
use std::collections::HashMap;

/// Minimum time between two celebrations; hits in between are merged.
pub const COOLDOWN_MS: u64 = 60_000;
/// Period key used for lifetime milestones.
pub const LIFETIME: &str = "life";

#[derive(Clone, Debug, PartialEq)]
pub struct MilestoneDef {
    pub id: String,
    pub period: Period,
    pub metric: Metric,
    pub threshold: f64,
    pub repeat: bool,
}

impl From<&CustomMilestone> for MilestoneDef {
    fn from(m: &CustomMilestone) -> Self {
        Self {
            id: m.id.clone(),
            period: m.period,
            metric: m.metric,
            threshold: m.threshold,
            repeat: m.repeat,
        }
    }
}

/// One built-in series. `name` goes into the ids, which are stored with what
/// has been celebrated, so it must not change: the count series is still
/// called "keys" from when it counted nothing else, and a new name would
/// celebrate every threshold already passed all over again.
fn fixed(
    period: Period,
    metric: Metric,
    name: &'static str,
    thresholds: &'static [f64],
) -> impl Iterator<Item = MilestoneDef> {
    thresholds.iter().map(move |&threshold| MilestoneDef {
        id: format!("builtin:{period:?}:{name}:{threshold}").to_lowercase(),
        period,
        metric,
        threshold,
        repeat: false,
    })
}

/// The built-in set, followed by the user's own. The built-in counts add up
/// what the head counter does, so a milestone lands when that number says so.
pub fn definitions(custom: &[CustomMilestone], head: &HeadCounter) -> Vec<MilestoneDef> {
    let counted = head.metric();
    fixed(Period::Daily, counted, "keys", &[1e3, 5e3, 1e4, 2e4, 5e4])
        .chain(fixed(Period::Lifetime, counted, "keys", &[1e5, 1e6, 1e7]))
        .chain(fixed(
            Period::Daily,
            Metric::Distance,
            "distance",
            &[100.0, 1000.0],
        ))
        .chain(custom.iter().map(MilestoneDef::from))
        .collect()
}

pub fn metric_value(t: &Totals, m: Metric) -> f64 {
    match m {
        Metric::Keys => t.keys as f64,
        Metric::Clicks => clicks(t) as f64,
        Metric::Inputs => (t.keys + clicks(t)) as f64,
        Metric::Scrolls => t.scrolls as f64,
        Metric::Distance => t.move_mm / 1000.0,
    }
}

fn clicks(t: &Totals) -> u64 {
    t.click_left + t.click_right + t.click_middle
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub id: String,
    pub period: Period,
    pub metric: Metric,
    /// The level reached: the threshold, or a multiple of it for repeating ones.
    pub level: f64,
}

/// Highest level celebrated so far per milestone, keyed by id, with the
/// period it belongs to (a date for daily ones, [`LIFETIME`] otherwise).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reached(pub HashMap<String, (String, f64)>);

impl Reached {
    /// Returns milestones newly crossed and records them.
    pub fn check(
        &mut self,
        defs: &[MilestoneDef],
        date: &str,
        today: &Totals,
        lifetime: &Totals,
    ) -> Vec<Hit> {
        let mut hits = Vec::new();
        for d in defs {
            let (period_key, totals) = match d.period {
                Period::Daily => (date, today),
                Period::Lifetime => (LIFETIME, lifetime),
            };
            let v = metric_value(totals, d.metric);
            let level = if d.repeat {
                (v / d.threshold).floor() * d.threshold
            } else if v >= d.threshold {
                d.threshold
            } else {
                0.0
            };
            if level < d.threshold {
                continue;
            }
            let last = match self.0.get(&d.id) {
                Some((p, l)) if p == period_key => *l,
                _ => 0.0,
            };
            if level > last {
                self.0.insert(d.id.clone(), (period_key.to_string(), level));
                hits.push(Hit {
                    id: d.id.clone(),
                    period: d.period,
                    metric: d.metric,
                    level,
                });
            }
        }
        hits
    }
}

/// Spaces celebrations at least [`COOLDOWN_MS`] apart, merging what piles up.
#[derive(Default)]
pub struct Celebrations {
    queue: Vec<Hit>,
    next_allowed_ms: u64,
}

impl Celebrations {
    pub fn push(&mut self, hits: Vec<Hit>) {
        self.queue.extend(hits);
    }

    /// Everything to celebrate now, if the cooldown allows.
    pub fn poll(&mut self, now_ms: u64) -> Option<Vec<Hit>> {
        if self.queue.is_empty() || now_ms < self.next_allowed_ms {
            return None;
        }
        self.next_allowed_ms = now_ms + COOLDOWN_MS;
        Some(std::mem::take(&mut self.queue))
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(n: u64) -> Totals {
        Totals {
            keys: n,
            ..Default::default()
        }
    }

    fn custom(threshold: f64, repeat: bool, period: Period) -> CustomMilestone {
        CustomMilestone {
            id: "c1".into(),
            period,
            metric: Metric::Keys,
            threshold,
            repeat,
        }
    }

    fn levels(hits: &[Hit]) -> Vec<f64> {
        hits.iter().map(|h| h.level).collect()
    }

    #[test]
    fn builtins_have_unique_ids() {
        let defs = definitions(&[], &HeadCounter::default());
        let mut ids: Vec<_> = defs.iter().map(|d| d.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), defs.len());
        assert_eq!(defs.len(), 10);
    }

    #[test]
    fn builtin_counts_add_up_what_the_head_counter_does() {
        let today = Totals {
            keys: 600,
            click_left: 300,
            click_right: 100,
            ..Default::default()
        };
        let head = |keyboard, mouse| HeadCounter {
            keyboard,
            mouse,
            ..Default::default()
        };
        let first = |head: HeadCounter| {
            let defs = definitions(&[], &head);
            let mut r = Reached::default();
            (
                defs[0].metric,
                levels(&r.check(&defs, "d", &today, &keys(0))),
            )
        };
        assert_eq!(first(head(true, true)), (Metric::Inputs, vec![1000.0]));
        assert_eq!(first(head(true, false)), (Metric::Keys, vec![]));
        assert_eq!(first(head(false, true)), (Metric::Clicks, vec![]));
        assert_eq!(first(head(false, false)), (Metric::Inputs, vec![1000.0]));
    }

    #[test]
    fn builtin_ids_do_not_depend_on_what_is_counted() {
        let ids = |keyboard, mouse| {
            let head = HeadCounter {
                keyboard,
                mouse,
                ..Default::default()
            };
            definitions(&[], &head)
                .into_iter()
                .map(|d| d.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(true, true), ids(true, false));
        assert_eq!(ids(true, true), ids(false, true));
        assert_eq!(ids(true, true)[0], "builtin:daily:keys:1000");
        assert_eq!(ids(true, true)[8], "builtin:daily:distance:100");
    }

    #[test]
    fn daily_fires_once_per_day_and_again_tomorrow() {
        let defs = definitions(&[], &HeadCounter::default());
        let mut r = Reached::default();
        let life = keys(0);
        assert_eq!(
            levels(&r.check(&defs, "2026-09-23", &keys(999), &life)),
            Vec::<f64>::new()
        );
        assert_eq!(
            levels(&r.check(&defs, "2026-09-23", &keys(1000), &life)),
            [1000.0]
        );
        assert!(r.check(&defs, "2026-09-23", &keys(1500), &life).is_empty());
        assert_eq!(
            levels(&r.check(&defs, "2026-09-24", &keys(1200), &life)),
            [1000.0]
        );
    }

    #[test]
    fn crossing_several_thresholds_at_once_reports_each() {
        let defs = definitions(&[], &HeadCounter::default());
        let mut r = Reached::default();
        let hits = r.check(&defs, "2026-09-23", &keys(12_000), &keys(0));
        assert_eq!(levels(&hits), [1000.0, 5000.0, 10_000.0]);
    }

    #[test]
    fn lifetime_fires_once_ever() {
        let defs = definitions(&[], &HeadCounter::default());
        let mut r = Reached::default();
        let hits = r.check(&defs, "2026-09-23", &keys(0), &keys(100_000));
        assert_eq!(levels(&hits), [100_000.0]);
        assert!(r
            .check(&defs, "2027-01-01", &keys(0), &keys(150_000))
            .is_empty());
    }

    #[test]
    fn repeating_milestone_skipping_steps_fires_once_at_the_highest() {
        let defs = definitions(
            &[custom(500.0, true, Period::Daily)],
            &HeadCounter::default(),
        );
        let custom_only = &defs[10..];
        let mut r = Reached::default();
        assert!(r.check(custom_only, "d", &keys(499), &keys(0)).is_empty());
        assert_eq!(
            levels(&r.check(custom_only, "d", &keys(500), &keys(0))),
            [500.0]
        );
        assert!(r.check(custom_only, "d", &keys(999), &keys(0)).is_empty());
        assert_eq!(
            levels(&r.check(custom_only, "d", &keys(2600), &keys(0))),
            [2500.0]
        );
        assert_eq!(
            levels(&r.check(custom_only, "d", &keys(3000), &keys(0))),
            [3000.0]
        );
    }

    #[test]
    fn distance_is_in_metres() {
        let defs = definitions(&[], &HeadCounter::default());
        let mut r = Reached::default();
        let today = Totals {
            move_mm: 100_000.0,
            ..Default::default()
        };
        let hits = r.check(&defs, "d", &today, &keys(0));
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].metric, hits[0].level), (Metric::Distance, 100.0));
    }

    #[test]
    fn raising_a_threshold_fires_again_but_lowering_does_not() {
        let mut r = Reached::default();
        let at = |t| {
            definitions(
                &[custom(t, false, Period::Lifetime)],
                &HeadCounter::default(),
            )[10..]
                .to_vec()
        };
        assert_eq!(
            levels(&r.check(&at(10_000.0), "d", &keys(0), &keys(12_000))),
            [10_000.0]
        );
        assert!(r
            .check(&at(5_000.0), "d", &keys(0), &keys(12_000))
            .is_empty());
        assert_eq!(
            levels(&r.check(&at(12_000.0), "d", &keys(0), &keys(12_000))),
            [12_000.0]
        );
    }

    #[test]
    fn celebrations_respect_the_cooldown_and_merge() {
        let hit = |level| Hit {
            id: "x".into(),
            period: Period::Daily,
            metric: Metric::Keys,
            level,
        };
        let mut c = Celebrations::default();
        assert_eq!(c.poll(0), None);
        c.push(vec![hit(1.0)]);
        assert_eq!(c.poll(1_000).map(|h| h.len()), Some(1));
        c.push(vec![hit(2.0)]);
        c.push(vec![hit(3.0)]);
        assert_eq!(c.poll(30_000), None, "still cooling down");
        assert_eq!(c.poll(61_000).map(|h| levels(&h)), Some(vec![2.0, 3.0]));
        assert_eq!(c.poll(200_000), None);
    }
}
