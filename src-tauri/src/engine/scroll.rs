//! Turns raw scroll events into "scroll gestures": a burst of scrolling with
//! no pause longer than [`GAP_MS`] counts once. Momentum events never count.

pub const GAP_MS: u64 = 300;

#[derive(Default)]
pub struct ScrollSegmenter {
    last_ms: Option<u64>,
}

impl ScrollSegmenter {
    /// Feeds one non-momentum scroll event; returns true when it starts a new gesture.
    pub fn feed(&mut self, t_ms: u64) -> bool {
        let new = self
            .last_ms
            .is_none_or(|last| t_ms.saturating_sub(last) > GAP_MS);
        self.last_ms = Some(t_ms);
        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gestures(times: &[u64]) -> usize {
        let mut s = ScrollSegmenter::default();
        times.iter().filter(|&&t| s.feed(t)).count()
    }

    #[test]
    fn continuous_scrolling_is_one_gesture() {
        assert_eq!(gestures(&[0, 50, 100, 150, 400, 700]), 1);
    }

    #[test]
    fn gap_of_exactly_300ms_continues_but_301ms_splits() {
        assert_eq!(gestures(&[0, 300]), 1);
        assert_eq!(gestures(&[0, 301]), 2);
    }

    #[test]
    fn separate_flicks_count_separately() {
        assert_eq!(gestures(&[0, 20, 40, 1000, 1020, 5000]), 3);
    }
}
