//! Rolling per-second rate over a short window, so the number reacts quickly.

const BUCKET_MS: u64 = 500;
const BUCKETS: usize = 10; // 5-second window
const WINDOW_SECS: f64 = (BUCKET_MS * BUCKETS as u64) as f64 / 1000.0;

#[derive(Default)]
pub struct RateWindow {
    counts: [u32; BUCKETS],
    /// Which bucket index (t / BUCKET_MS) each slot currently holds.
    slots: [u64; BUCKETS],
}

impl RateWindow {
    pub fn record(&mut self, t_ms: u64) {
        let bucket = t_ms / BUCKET_MS;
        let i = (bucket % BUCKETS as u64) as usize;
        if self.slots[i] != bucket {
            self.slots[i] = bucket;
            self.counts[i] = 0;
        }
        self.counts[i] += 1;
    }

    /// Events per second over the last 5 seconds, to one decimal.
    pub fn per_second(&self, t_ms: u64) -> f64 {
        let now = t_ms / BUCKET_MS;
        let recent: u64 = self
            .slots
            .iter()
            .zip(self.counts)
            .filter(|(&slot, _)| slot <= now && now - slot < BUCKETS as u64)
            .map(|(_, c)| c as u64)
            .sum();
        (recent as f64 / WINDOW_SECS * 10.0).round() / 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_typing_reports_its_rate() {
        let mut r = RateWindow::default();
        // 5 keys per second for 5 seconds.
        for i in 0..25 {
            r.record(1_000 + i * 200);
        }
        assert_eq!(r.per_second(5_999), 5.0);
    }

    #[test]
    fn rate_decays_to_zero_after_the_window() {
        let mut r = RateWindow::default();
        r.record(1_000);
        assert_eq!(r.per_second(1_000), 0.2);
        assert_eq!(r.per_second(5_999), 0.2);
        assert_eq!(r.per_second(6_000), 0.0);
    }

    #[test]
    fn stale_slots_are_reused() {
        let mut r = RateWindow::default();
        r.record(0);
        r.record(5_000); // same slot, 10 buckets later
        assert_eq!(r.per_second(5_000), 0.2);
    }
}
