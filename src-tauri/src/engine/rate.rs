//! Rolling "per minute" rate over a short window, so the number reacts quickly.

const BUCKET_MS: u64 = 500;
const BUCKETS: usize = 20; // 10-second window
const PER_MINUTE: u64 = 60_000 / (BUCKET_MS * BUCKETS as u64);

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

    /// Events in the last 10 seconds, scaled to a per-minute rate.
    pub fn per_minute(&self, t_ms: u64) -> u32 {
        let now = t_ms / BUCKET_MS;
        let recent: u64 = self
            .slots
            .iter()
            .zip(self.counts)
            .filter(|(&slot, _)| slot <= now && now - slot < BUCKETS as u64)
            .map(|(_, c)| c as u64)
            .sum();
        (recent * PER_MINUTE) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_typing_reports_its_rate() {
        let mut r = RateWindow::default();
        // 5 keys per second for 10 seconds = 300 per minute.
        for i in 0..50 {
            r.record(1_000 + i * 200);
        }
        assert_eq!(r.per_minute(10_999), 300);
    }

    #[test]
    fn rate_decays_to_zero_after_the_window() {
        let mut r = RateWindow::default();
        r.record(1_000);
        assert_eq!(r.per_minute(1_000), 6);
        assert_eq!(r.per_minute(10_999), 6);
        assert_eq!(r.per_minute(11_000), 0);
    }

    #[test]
    fn stale_slots_are_reused() {
        let mut r = RateWindow::default();
        r.record(0);
        r.record(10_000); // same slot, 20 buckets later
        assert_eq!(r.per_minute(10_000), 6);
    }
}
