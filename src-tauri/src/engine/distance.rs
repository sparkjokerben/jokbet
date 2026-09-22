//! Mouse travel in raw units and in millimetres, using each display's physical size.

/// 96 DPI, used when a display's physical size is unknown or implausible.
pub const DEFAULT_MM_PER_UNIT: f64 = 25.4 / 96.0;
/// A single step longer than this is a cursor warp (display switch, remote
/// desktop), not movement.
const MAX_STEP: f64 = 1500.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Display {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub mm_per_unit: f64,
}

impl Display {
    /// Builds a display from its bounds (in event units) and physical width in mm.
    pub fn new(x: f64, y: f64, w: f64, h: f64, width_mm: f64) -> Self {
        let ratio = width_mm / w;
        let mm_per_unit = if (0.05..=0.6).contains(&ratio) {
            ratio
        } else {
            DEFAULT_MM_PER_UNIT
        };
        Self {
            x,
            y,
            w,
            h,
            mm_per_unit,
        }
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Clone, Debug, Default)]
pub struct DisplayMap(pub Vec<Display>);

impl DisplayMap {
    pub fn mm_per_unit(&self, x: f64, y: f64) -> f64 {
        self.0
            .iter()
            .find(|d| d.contains(x, y))
            .map_or(DEFAULT_MM_PER_UNIT, |d| d.mm_per_unit)
    }
}

#[derive(Default)]
pub struct DistanceTracker {
    last: Option<(f64, f64)>,
}

impl DistanceTracker {
    /// Returns the distance moved since the last position as (units, mm).
    pub fn feed(&mut self, x: f64, y: f64, displays: &DisplayMap) -> (f64, f64) {
        let prev = self.last.replace((x, y));
        let Some((px, py)) = prev else {
            return (0.0, 0.0);
        };
        let d = (x - px).hypot(y - py);
        if d > MAX_STEP {
            return (0.0, 0.0);
        }
        (d, d * displays.mm_per_unit((x + px) / 2.0, (y + py) / 2.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_displays() -> DisplayMap {
        DisplayMap(vec![
            Display::new(0.0, 0.0, 1000.0, 800.0, 200.0), // 0.2 mm/unit
            Display::new(1000.0, 0.0, 1000.0, 800.0, 400.0), // 0.4 mm/unit
        ])
    }

    #[test]
    fn first_sample_has_no_distance() {
        let mut t = DistanceTracker::default();
        assert_eq!(t.feed(10.0, 10.0, &two_displays()), (0.0, 0.0));
    }

    #[test]
    fn distance_uses_the_display_under_the_cursor() {
        let map = two_displays();
        let mut t = DistanceTracker::default();
        t.feed(100.0, 100.0, &map);
        let (u, mm) = t.feed(103.0, 104.0, &map);
        assert_eq!(u, 5.0);
        assert!((mm - 1.0).abs() < 1e-9);

        t.feed(1500.0, 100.0, &map);
        let (u, mm) = t.feed(1510.0, 100.0, &map);
        assert_eq!(u, 10.0);
        assert!((mm - 4.0).abs() < 1e-9);
    }

    #[test]
    fn warps_are_ignored() {
        let map = two_displays();
        let mut t = DistanceTracker::default();
        t.feed(0.0, 0.0, &map);
        assert_eq!(t.feed(1900.0, 700.0, &map), (0.0, 0.0));
        assert_eq!(t.feed(1901.0, 700.0, &map).0, 1.0);
    }

    #[test]
    fn implausible_physical_size_falls_back_to_96_dpi() {
        assert_eq!(
            Display::new(0.0, 0.0, 1000.0, 800.0, 0.0).mm_per_unit,
            DEFAULT_MM_PER_UNIT
        );
        assert_eq!(
            Display::new(0.0, 0.0, 1000.0, 800.0, 5000.0).mm_per_unit,
            DEFAULT_MM_PER_UNIT
        );
    }

    #[test]
    fn off_display_points_use_the_default() {
        assert_eq!(two_displays().mm_per_unit(-5.0, -5.0), DEFAULT_MM_PER_UNIT);
    }
}
