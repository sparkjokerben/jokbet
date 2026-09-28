//! The break reminder: how long someone has been at it without a pause, and
//! when the pet should say so. Pure; the runtime hands it the clock.
//!
//! A stretch is unbroken as long as no gap between one key, click or scroll
//! and the next is longer than the configured gap; a longer gap is a rest, and
//! the next input starts a new stretch. Once a stretch is long enough the pet
//! reminds, and then again every so often until there is a rest.

/// How long after a reminder, or after the user waves it away, the next one
/// comes if they keep going.
pub const REMIND_AGAIN_MS: u64 = 15 * 60_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RestConfig {
    pub enabled: bool,
    /// A pause longer than this is a rest.
    pub gap_ms: u64,
    /// A stretch this long gets a reminder.
    pub after_ms: u64,
}

impl From<crate::settings::RestReminder> for RestConfig {
    fn from(r: crate::settings::RestReminder) -> Self {
        Self {
            enabled: r.enabled,
            gap_ms: u64::from(r.gap_min) * 60_000,
            after_ms: u64::from(r.after_min) * 60_000,
        }
    }
}

#[derive(Debug, Default)]
pub struct RestTracker {
    cfg: RestConfig,
    /// When the current stretch began; `None` while resting.
    start: Option<u64>,
    /// The latest input of the stretch.
    last_active: u64,
    /// When the next reminder is due.
    due: u64,
    /// Whether this stretch has had a reminder yet.
    reminded: bool,
    /// The previous poll, which is how long a pause lasted.
    last_poll: Option<u64>,
}

impl RestTracker {
    pub fn new(cfg: RestConfig) -> Self {
        Self {
            cfg,
            ..Self::default()
        }
    }

    pub fn configure(&mut self, cfg: RestConfig) {
        if !cfg.enabled {
            *self = Self::new(cfg);
            return;
        }
        self.cfg = cfg;
        // A new length applies to the stretch under way, until it has had
        // its first reminder; after that the reminders keep their own pace.
        if let (Some(start), false) = (self.start, self.reminded) {
            self.due = start + cfg.after_ms;
        }
    }

    /// A key, click or scroll at `t`. None of it counts while counting is
    /// paused.
    pub fn activity(&mut self, t: u64, paused: bool) {
        if !self.cfg.enabled || paused {
            return;
        }
        let rested = t.saturating_sub(self.last_active) > self.cfg.gap_ms;
        if self.start.is_none() || rested {
            self.start = Some(t);
            self.due = t + self.cfg.after_ms;
            self.reminded = false;
        }
        self.last_active = self.last_active.max(t);
    }

    /// Looks at the clock, about once a second. Returns how many minutes the
    /// stretch has lasted when it is time to remind. While counting is paused
    /// the stretch is frozen, neither growing nor running out; while the pet
    /// is off screen a reminder waits for it to be back.
    pub fn poll(&mut self, now: u64, paused: bool, shown: bool) -> Option<u32> {
        let since = self
            .last_poll
            .replace(now)
            .map_or(0, |prev| now.saturating_sub(prev));
        let start = self.start?;
        if paused {
            self.start = Some(start + since);
            self.last_active += since;
            self.due += since;
            return None;
        }
        if now.saturating_sub(self.last_active) > self.cfg.gap_ms {
            self.start = None;
            self.reminded = false;
            return None;
        }
        if now < self.due || !shown {
            return None;
        }
        self.reminded = true;
        self.due = now + REMIND_AGAIN_MS;
        Some((now.saturating_sub(start) / 60_000) as u32)
    }

    /// The user has seen the reminder: the next one waits the full interval
    /// from now. Before any reminder there is nothing to put off.
    pub fn ack(&mut self, now: u64) {
        if self.start.is_some() && self.reminded {
            self.due = now + REMIND_AGAIN_MS;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    fn tracker() -> RestTracker {
        RestTracker::new(RestConfig {
            enabled: true,
            gap_ms: 5 * MIN,
            after_ms: 50 * MIN,
        })
    }

    /// Input every `every` ms from `from` to `to`, polling after each.
    fn work(r: &mut RestTracker, from: u64, to: u64, every: u64) -> Vec<(u64, u32)> {
        let mut fired = Vec::new();
        let mut t = from;
        while t <= to {
            r.activity(t, false);
            if let Some(min) = r.poll(t, false, true) {
                fired.push((t, min));
            }
            t += every;
        }
        fired
    }

    #[test]
    fn a_long_stretch_gets_a_reminder_with_its_length() {
        let mut r = tracker();
        assert_eq!(work(&mut r, 0, 49 * MIN, MIN), []);
        assert_eq!(work(&mut r, 50 * MIN, 50 * MIN, MIN), [(50 * MIN, 50)]);
    }

    #[test]
    fn short_pauses_keep_the_stretch_and_a_long_one_is_a_rest() {
        let mut r = tracker();
        work(&mut r, 0, 20 * MIN, MIN);
        // Four and a half minutes away: still the same stretch.
        let fired = work(&mut r, 24 * MIN + 30_000, 51 * MIN, MIN);
        assert_eq!(fired.first().map(|f| f.1), Some(50));

        let mut r = tracker();
        work(&mut r, 0, 40 * MIN, MIN);
        assert_eq!(r.poll(46 * MIN, false, true), None);
        // Six minutes away was a rest: the next stretch starts from zero.
        assert_eq!(work(&mut r, 46 * MIN, 95 * MIN, MIN), []);
        assert_eq!(work(&mut r, 96 * MIN, 96 * MIN, MIN), [(96 * MIN, 50)]);
    }

    #[test]
    fn it_reminds_again_every_quarter_hour_until_a_rest() {
        let mut r = tracker();
        let fired = work(&mut r, 0, 90 * MIN, MIN);
        assert_eq!(fired, [(50 * MIN, 50), (65 * MIN, 65), (80 * MIN, 80)]);
    }

    #[test]
    fn waving_it_away_restarts_the_wait() {
        let mut r = tracker();
        work(&mut r, 0, 50 * MIN, MIN);
        r.ack(58 * MIN);
        let fired = work(&mut r, 51 * MIN, 80 * MIN, MIN);
        assert_eq!(fired, [(73 * MIN, 73)]);
    }

    #[test]
    fn a_click_before_any_reminder_changes_nothing() {
        let mut r = tracker();
        work(&mut r, 0, 30 * MIN, MIN);
        r.ack(30 * MIN);
        assert_eq!(work(&mut r, 31 * MIN, 50 * MIN, MIN), [(50 * MIN, 50)]);
    }

    #[test]
    fn a_pause_freezes_the_stretch() {
        let mut r = tracker();
        work(&mut r, 0, 30 * MIN, MIN);
        // An hour paused, still typing: nothing counts, nothing runs out.
        let mut t = 31 * MIN;
        while t <= 90 * MIN {
            r.activity(t, true);
            assert_eq!(r.poll(t, true, true), None);
            t += MIN;
        }
        // Twenty more minutes after it make fifty.
        let fired = work(&mut r, 91 * MIN, 120 * MIN, MIN);
        assert_eq!(fired.first(), Some(&(110 * MIN, 50)));
    }

    #[test]
    fn a_hidden_pet_saves_the_reminder_for_later() {
        let mut r = tracker();
        work(&mut r, 0, 49 * MIN, MIN);
        for t in 50..=55 {
            r.activity(t * MIN, false);
            assert_eq!(r.poll(t * MIN, false, false), None);
        }
        assert_eq!(r.poll(55 * MIN + 1000, false, true), Some(55));
    }

    #[test]
    fn a_rest_while_hidden_drops_the_reminder() {
        let mut r = tracker();
        work(&mut r, 0, 50 * MIN - 1, MIN);
        assert_eq!(r.poll(52 * MIN, false, false), None);
        assert_eq!(r.poll(60 * MIN, false, true), None);
    }

    #[test]
    fn switched_off_it_says_nothing() {
        let mut r = RestTracker::new(RestConfig::default());
        assert_eq!(work(&mut r, 0, 200 * MIN, MIN), []);
        let mut r = tracker();
        work(&mut r, 0, 30 * MIN, MIN);
        r.configure(RestConfig::default());
        assert_eq!(work(&mut r, 31 * MIN, 200 * MIN, MIN), []);
    }

    #[test]
    fn a_new_length_applies_to_the_stretch_under_way() {
        let mut r = tracker();
        work(&mut r, 0, 20 * MIN, MIN);
        r.configure(RestConfig {
            enabled: true,
            gap_ms: 5 * MIN,
            after_ms: 25 * MIN,
        });
        assert_eq!(work(&mut r, 21 * MIN, 25 * MIN, MIN), [(25 * MIN, 25)]);
    }
}
