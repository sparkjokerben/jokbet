//! Turns raw input events into today's counters. Pure: the caller supplies the
//! clock, so an event is counted into whichever hour it is handed.

use super::distance::{DisplayMap, DistanceTracker};
use super::rate::RateWindow;
use super::scroll::ScrollSegmenter;
use crate::input::{MouseButton, RawEvent, TimedEvent};
use serde::Serialize;
use std::collections::HashMap;

/// A key-down this soon after the previous one for a key still held is an
/// auto-repeat (Windows hooks repeat key-downs); after longer, a key-up was missed.
const REPEAT_WINDOW_MS: u64 = 1100;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub keys: u64,
    pub click_left: u64,
    pub click_right: u64,
    pub click_middle: u64,
    pub scrolls: u64,
    pub move_px: f64,
    pub move_mm: f64,
}

impl Totals {
    pub fn add(&mut self, o: &Totals) {
        self.keys += o.keys;
        self.click_left += o.click_left;
        self.click_right += o.click_right;
        self.click_middle += o.click_middle;
        self.scrolls += o.scrolls;
        self.move_px += o.move_px;
        self.move_mm += o.move_mm;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayCounters {
    pub totals: Totals,
    pub per_key: HashMap<String, u64>,
    /// Totals per hour of the day, so a day can say when it was busy. It rides
    /// along with the counters it belongs to, which is what keeps merges, day
    /// rollovers and retried flushes from dropping the hour.
    pub per_hour: [Totals; 24],
}

impl DayCounters {
    /// Adds to the day's totals and to the hour it happened in. `hour` is
    /// 0..24 from the caller's clock; anything else updates the totals alone.
    fn apply(&mut self, hour: u8, f: impl Fn(&mut Totals)) {
        f(&mut self.totals);
        if let Some(hourly) = self.per_hour.get_mut(hour as usize) {
            f(hourly);
        }
    }

    /// Every hourly bucket is written by `apply`, which touches `totals` in the
    /// same call, so an empty `totals` implies empty hours. The flush skips work
    /// on this, and moving a count off `apply` would quietly break that.
    pub fn is_empty(&self) -> bool {
        self.totals == Totals::default() && self.per_key.is_empty()
    }

    pub fn merge(&mut self, other: DayCounters) {
        self.totals.add(&other.totals);
        for (key, n) in other.per_key {
            *self.per_key.entry(key).or_default() += n;
        }
        for (mine, theirs) in self.per_hour.iter_mut().zip(other.per_hour.iter()) {
            mine.add(theirs);
        }
    }

    fn bump_key(&mut self, code: &str, hour: u8) {
        self.apply(hour, |t| t.keys += 1);
        match self.per_key.get_mut(code) {
            Some(n) => *n += 1,
            None => {
                self.per_key.insert(code.to_owned(), 1);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Activity {
    Typing,
    Click,
}

#[derive(Default)]
pub struct Aggregator {
    /// Everything counted today, including what was already persisted.
    pub today: DayCounters,
    /// Counted but not yet written to disk.
    pub pending: DayCounters,
    pub paused: bool,
    held: HashMap<&'static str, u64>,
    scroll: ScrollSegmenter,
    keys_rate: RateWindow,
    clicks_rate: RateWindow,
    distance: DistanceTracker,
}

impl Aggregator {
    /// Starts a day from already-persisted counters.
    pub fn with_today(today: DayCounters) -> Self {
        Self {
            today,
            ..Self::default()
        }
    }

    /// Counts one event; returns the kind of activity the pet should react to.
    /// `hour` is the local hour the event happened in, 0..24.
    pub fn ingest(&mut self, e: TimedEvent, displays: &DisplayMap, hour: u8) -> Option<Activity> {
        match e.ev {
            RawEvent::Key { code, down: true } => {
                let prev = self.held.insert(code, e.t_ms);
                if prev.is_some_and(|p| e.t_ms.saturating_sub(p) < REPEAT_WINDOW_MS) {
                    return None;
                }
                if !self.paused {
                    self.keys_rate.record(e.t_ms);
                    self.today.bump_key(code, hour);
                    self.pending.bump_key(code, hour);
                }
                Some(Activity::Typing)
            }
            RawEvent::Key { code, down: false } => {
                self.held.remove(code);
                None
            }
            RawEvent::Button { button, down: true } => {
                if !self.paused {
                    let bump: fn(&mut Totals) = match button {
                        MouseButton::Left => |t| t.click_left += 1,
                        MouseButton::Right => |t| t.click_right += 1,
                        MouseButton::Middle => |t| t.click_middle += 1,
                        MouseButton::Other => |_| {},
                    };
                    if button != MouseButton::Other {
                        self.clicks_rate.record(e.t_ms);
                    }
                    self.today.apply(hour, bump);
                    self.pending.apply(hour, bump);
                }
                Some(Activity::Click)
            }
            RawEvent::Button { down: false, .. } => None,
            RawEvent::Scroll { momentum: true } => None,
            // Scrolls are counted but do not change what the pet is doing.
            RawEvent::Scroll { momentum: false } => {
                if self.scroll.feed(e.t_ms) && !self.paused {
                    self.today.apply(hour, |t| t.scrolls += 1);
                    self.pending.apply(hour, |t| t.scrolls += 1);
                }
                None
            }
            RawEvent::Move { x, y } => {
                let (px, mm) = self.distance.feed(x, y, displays);
                if !self.paused && px > 0.0 {
                    for day in [&mut self.today, &mut self.pending] {
                        day.apply(hour, |t| {
                            t.move_px += px;
                            t.move_mm += mm;
                        });
                    }
                }
                None
            }
        }
    }

    pub fn keys_per_second(&self, t_ms: u64) -> f64 {
        self.keys_rate.per_second(t_ms)
    }

    pub fn clicks_per_second(&self, t_ms: u64) -> f64 {
        self.clicks_rate.per_second(t_ms)
    }

    /// Hands over what still needs saving.
    pub fn take_pending(&mut self) -> DayCounters {
        std::mem::take(&mut self.pending)
    }

    /// Starts a new day; returns the old day's unsaved counters.
    pub fn roll_over(&mut self) -> DayCounters {
        self.today = DayCounters::default();
        self.take_pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::distance::Display;

    fn ev(t_ms: u64, ev: RawEvent) -> TimedEvent {
        TimedEvent { t_ms, ev }
    }
    fn key(t: u64, code: &'static str, down: bool) -> TimedEvent {
        ev(t, RawEvent::Key { code, down })
    }
    fn click(t: u64, button: MouseButton) -> [TimedEvent; 2] {
        [
            ev(t, RawEvent::Button { button, down: true }),
            ev(
                t + 50,
                RawEvent::Button {
                    button,
                    down: false,
                },
            ),
        ]
    }
    fn feed(a: &mut Aggregator, events: impl IntoIterator<Item = TimedEvent>) {
        feed_at(a, 12, events);
    }
    fn feed_at(a: &mut Aggregator, hour: u8, events: impl IntoIterator<Item = TimedEvent>) {
        let map = DisplayMap(vec![Display::new(0.0, 0.0, 1000.0, 1000.0, 200.0)]);
        for e in events {
            a.ingest(e, &map, hour);
        }
    }

    #[test]
    fn counts_keys_per_code() {
        let mut a = Aggregator::default();
        feed(
            &mut a,
            [
                key(0, "KeyA", true),
                key(80, "KeyA", false),
                key(100, "KeyB", true),
                key(150, "KeyB", false),
                key(200, "KeyA", true),
                key(260, "KeyA", false),
            ],
        );
        assert_eq!(a.today.totals.keys, 3);
        assert_eq!(a.today.per_key["KeyA"], 2);
        assert_eq!(a.today.per_key["KeyB"], 1);
        assert_eq!(a.pending, a.today);
    }

    #[test]
    fn repeated_key_downs_while_held_count_once() {
        let mut a = Aggregator::default();
        feed(&mut a, (0..20).map(|i| key(500 + i * 33, "KeyJ", true)));
        feed(&mut a, [key(2000, "KeyJ", false)]);
        assert_eq!(a.today.totals.keys, 1);
    }

    #[test]
    fn missed_key_up_does_not_swallow_later_presses() {
        let mut a = Aggregator::default();
        feed(&mut a, [key(0, "KeyK", true), key(5000, "KeyK", true)]);
        assert_eq!(a.today.totals.keys, 2);
    }

    #[test]
    fn clicks_by_button_and_other_buttons_are_not_counted() {
        let mut a = Aggregator::default();
        feed(&mut a, click(0, MouseButton::Left));
        feed(&mut a, click(100, MouseButton::Left));
        feed(&mut a, click(200, MouseButton::Right));
        feed(&mut a, click(300, MouseButton::Middle));
        feed(&mut a, click(400, MouseButton::Other));
        let t = &a.today.totals;
        assert_eq!((t.click_left, t.click_right, t.click_middle), (2, 1, 1));
    }

    #[test]
    fn scroll_gestures_ignore_momentum() {
        let mut a = Aggregator::default();
        let scroll = |t, momentum| ev(t, RawEvent::Scroll { momentum });
        feed(
            &mut a,
            [
                scroll(0, false),
                scroll(16, false),
                scroll(32, false),
                scroll(400, true),
                scroll(800, true),
                scroll(1200, false),
            ],
        );
        assert_eq!(a.today.totals.scrolls, 2);
    }

    #[test]
    fn movement_accumulates_units_and_mm() {
        let mut a = Aggregator::default();
        let mv = |t, x, y| ev(t, RawEvent::Move { x, y });
        feed(
            &mut a,
            [mv(0, 0.0, 0.0), mv(10, 30.0, 40.0), mv(20, 30.0, 140.0)],
        );
        assert_eq!(a.today.totals.move_px, 150.0);
        assert!((a.today.totals.move_mm - 30.0).abs() < 1e-9);
    }

    #[test]
    fn paused_reacts_but_does_not_count() {
        let mut a = Aggregator {
            paused: true,
            ..Default::default()
        };
        let map = DisplayMap::default();
        assert_eq!(
            a.ingest(key(0, "KeyA", true), &map, 12),
            Some(Activity::Typing)
        );
        feed(&mut a, click(10, MouseButton::Left));
        assert!(a.today.is_empty());
        assert_eq!(a.keys_per_second(100), 0.0);
    }

    #[test]
    fn roll_over_hands_back_pending_and_resets_today() {
        let mut a = Aggregator::with_today(DayCounters {
            totals: Totals {
                keys: 100,
                ..Default::default()
            },
            ..Default::default()
        });
        feed(&mut a, [key(0, "KeyA", true)]);
        assert_eq!(a.today.totals.keys, 101);
        let old = a.roll_over();
        assert_eq!(old.totals.keys, 1);
        assert!(a.today.is_empty() && a.pending.is_empty());
        feed(&mut a, [key(100, "KeyA", false), key(200, "KeyA", true)]);
        assert_eq!(a.today.totals.keys, 1);
    }

    #[test]
    fn rates_follow_recent_activity() {
        let mut a = Aggregator::default();
        feed(
            &mut a,
            (0..10).flat_map(|i| [key(i * 100, "KeyA", true), key(i * 100 + 50, "KeyA", false)]),
        );
        assert_eq!(a.keys_per_second(1000), 2.0);
        assert_eq!(a.clicks_per_second(1000), 0.0);
    }

    #[test]
    fn counts_by_hour_of_day() {
        let mut a = Aggregator::default();
        feed_at(
            &mut a,
            9,
            [
                key(0, "KeyA", true),
                key(50, "KeyA", false),
                key(100, "KeyB", true),
            ],
        );
        feed_at(&mut a, 14, click(200, MouseButton::Left));
        // Both the day and the unsaved delta carry the hour.
        for day in [&a.today, &a.pending] {
            assert_eq!(day.per_hour[9].keys, 2);
            assert_eq!(day.per_hour[14].click_left, 1);
            assert_eq!(day.per_hour[14].keys, 0);
            assert_eq!(day.per_hour[3], Totals::default());
        }
        assert_eq!(a.today.totals.keys, 2);
        assert_eq!(a.pending.totals.keys, 2);
    }

    #[test]
    fn movement_is_counted_by_hour_too() {
        let mut a = Aggregator::default();
        let mv = |t, x, y| ev(t, RawEvent::Move { x, y });
        feed_at(&mut a, 8, [mv(0, 0.0, 0.0), mv(10, 30.0, 40.0)]);
        // 50 px across the diagonal; the hourly bucket agrees with the day.
        assert_eq!(a.today.per_hour[8].move_px, 50.0);
        assert_eq!(a.today.per_hour[8].move_mm, a.today.totals.move_mm);
        assert!(a.today.per_hour[8].move_mm > 0.0);
        assert_eq!(a.pending.per_hour[8].move_px, 50.0);
    }

    #[test]
    fn merging_keeps_the_hourly_buckets() {
        let mut left = DayCounters::default();
        left.apply(20, |t| t.keys += 2);
        let mut right = DayCounters::default();
        right.apply(20, |t| t.keys += 3);
        right.apply(9, |t| t.click_left += 1);
        left.merge(right);
        assert_eq!(left.totals.keys, 5);
        assert_eq!(left.per_hour[20].keys, 5);
        assert_eq!(left.per_hour[9].click_left, 1);
    }

    #[test]
    fn roll_over_leaves_the_hourly_buckets_behind() {
        let mut a = Aggregator::default();
        feed_at(&mut a, 23, [key(0, "KeyA", true)]);
        let old = a.roll_over();
        assert_eq!(old.per_hour[23].keys, 1);
        assert_eq!(a.today.per_hour[23], Totals::default());
    }
}
