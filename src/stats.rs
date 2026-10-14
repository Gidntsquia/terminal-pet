//! Session stats for the pet in this terminal: age since launch and how often it was fed,
//! petted, booped, ruffled and dragged. Kept in memory only (nothing is written to disk),
//! shown on the `i` info card and as the age in the footer.

/// Counters for one session; everything starts at 0 on launch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// Seconds since launch.
    pub age: f32,
    pub fed: u32,
    pub petted: u32,
    pub booped: u32,
    /// Seconds the pet was reacting to the scroll-wheel ruffle.
    pub ruffled: f32,
    /// Seconds the pet was reacting to a drag (scratch / scritch / cuddle).
    pub dragged: f32,
    /// Seconds left of the host's drag reaction (like a guest's `Touch::stroke`).
    pub drag_left: f32,
}

impl Stats {
    /// Frame update: `ruffling` is whether the ruffle effect is active this frame.
    pub fn update(&mut self, dt: f32, ruffling: bool) {
        self.age += dt;
        if ruffling {
            self.ruffled += dt;
        }
        if self.drag_left > 0.0 {
            self.dragged += dt;
            self.drag_left = (self.drag_left - dt).max(0.0);
        }
    }
}

/// `42s`, `3m 12s`, `2h 14m 05s`, `1d 02h 14m 05s` (seconds rounded down).
pub fn long(secs: f32) -> String {
    let s = secs.max(0.0) as u64;
    let (d, h, m, s) = (s / 86400, s / 3600 % 24, s / 60 % 60, s % 60);
    if d > 0 {
        format!("{d}d {h:02}h {m:02}m {s:02}s")
    } else if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

/// Seconds for a gesture total: whole seconds rounded to nearest, `42s` / `3m 12s` / `1h 02m 03s`.
pub fn secs(secs: f32) -> String {
    long(secs.max(0.0).round())
}

/// Footer age: `0m 07s` under an hour, `2h 14m`, `1d 02h` past a day.
pub fn short(secs: f32) -> String {
    let s = secs.max(0.0) as u64;
    let (d, h, m, s) = (s / 86400, s / 3600 % 24, s / 60 % 60, s % 60);
    if d > 0 {
        format!("{d}d {h:02}h")
    } else if h > 0 {
        format!("{h}h {m:02}m")
    } else {
        format!("{m}m {s:02}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(long(0.0), "0s");
        assert_eq!(long(42.9), "42s");
        assert_eq!(long(192.0), "3m 12s");
        assert_eq!(long(2.0 * 3600.0 + 14.0 * 60.0 + 5.0), "2h 14m 05s");
        assert_eq!(
            long(86400.0 + 2.0 * 3600.0 + 14.0 * 60.0 + 5.0),
            "1d 02h 14m 05s"
        );
        assert_eq!(secs(0.8), "1s");
        assert_eq!(secs(192.4), "3m 12s");
        assert_eq!(short(7.0), "0m 07s");
        assert_eq!(short(2.0 * 3600.0 + 14.0 * 60.0 + 5.0), "2h 14m");
        assert_eq!(short(86400.0 + 2.0 * 3600.0 + 59.0), "1d 02h");
    }

    #[test]
    fn update_counts_only_active_effects() {
        let mut s = Stats::default();
        s.update(1.0, false);
        assert_eq!((s.age, s.ruffled, s.dragged), (1.0, 0.0, 0.0));
        s.update(0.5, true);
        s.drag_left = 1.0;
        s.update(0.5, false);
        s.update(0.5, false);
        s.update(0.5, false);
        assert_eq!(s.ruffled, 0.5);
        assert_eq!(s.dragged, 1.0);
        assert_eq!(s.age, 3.0);
    }
}
