// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains some utils for the whole solver module

use std::{fmt, time::Duration};

use ayto::matching_repr::MaskedMatching;

/// Entropy calculation for a candidate `m` across `left_poss`.
pub(super) fn calc_entropy(m: &MaskedMatching, left_poss: &[MaskedMatching]) -> f64 {
    let total = left_poss.len() as f64;

    let mut lights = [0u32; 11];
    for p in left_poss {
        // assume:
        // - p is the solution
        // - m is how they sit in the night
        let l = m.calculate_lights(p);
        lights[l as usize] += 1;
    }

    lights
        .into_iter()
        .filter(|&i| i > 0)
        .map(|i| {
            let p = (i as f64) / total;
            -p * p.log2()
        })
        .sum()
}

/// Collects simple runtime statistics for a sequence of duration samples.
///
/// Tracks:
/// - minimum observed runtime
/// - maximum observed runtime
/// - total accumulated runtime
/// - number of samples
#[derive(Debug, Clone)]
pub(super) struct RuntimeStats {
    /// the minimum observed duration
    min: Duration,
    /// the maximum observed duration
    max: Duration,
    /// amount of samples in the calculation
    count: usize,
    /// the total observed duration
    total: Duration,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            min: Duration::MAX,
            max: Duration::ZERO,
            count: 0,
            total: Duration::ZERO,
        }
    }
}

impl RuntimeStats {
    /// Adds a new duration sample to the statistics.
    pub(super) fn update(&mut self, d: Duration) {
        self.count +=1;
        self.total += d;
        self.min = self.min.min(d);
        self.max = self.max.max(d);
    }

    /// Returns the average runtime of all samples.
    ///
    /// If no samples have been recorded, `Duration::ZERO` is returned.
    fn avg(&self) -> Duration {
        if self.count == 0 {
            Duration::ZERO
        } else {
            self.total / self.count as u32
        }
    }
}

/// Formats a [`Duration`] into a compact human-readable representation.
///
/// Units are chosen dynamically:
///
/// |  Range | Unit |
/// |--------|------|
/// | >= 60s | minutes |
/// | >= 1s  | seconds |
/// | >= 1ms | milliseconds |
/// | <  1ms | microseconds |
fn fmt_duration(d: Duration) -> String {
    let s = d.as_secs_f64();

    if s >= 60.0 {
        format!("{:.2}m", s / 60.0)
    } else if s >= 1.0 {
        format!("{:.2}s", s)
    } else if s >= 0.001 {
        format!("{:.2}ms", s * 1000.0)
    } else {
        format!("{:.0}\u{00B5}s", s * 1_000_000.0)
    }
}

impl fmt::Display for RuntimeStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.count == 0 {
            write!(f, "no samples")
        } else {
            write!(
                f,
                "min={} avg={} max={}",
                fmt_duration(self.min),
                fmt_duration(self.avg()),
                fmt_duration(self.max),
            )
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use ayto::matching_repr::bitset::Bitset;
    use ayto::matching_repr::MaskedMatching;

    use pretty_assertions::assert_eq;
    use smallvec::SmallVec;

    #[test]
    fn default_has_no_samples() {
        let stats = RuntimeStats::default();

        assert_eq!(stats.count, 0);
        assert_eq!(stats.total, Duration::ZERO);
        assert_eq!(stats.min, Duration::MAX);
        assert_eq!(stats.max, Duration::ZERO);
        assert_eq!(stats.avg(), Duration::ZERO);
    }

    #[test]
    fn single_update_sets_all_fields() {
        let mut stats = RuntimeStats::default();
        let d = Duration::from_millis(42);

        stats.update(d);

        assert_eq!(stats.count, 1);
        assert_eq!(stats.total, d);
        assert_eq!(stats.min, d);
        assert_eq!(stats.max, d);
        assert_eq!(stats.avg(), d);
    }

    #[test]
    fn multiple_updates_compute_correct_stats() {
        let mut stats = RuntimeStats::default();

        stats.update(Duration::from_millis(10));
        stats.update(Duration::from_millis(30));
        stats.update(Duration::from_millis(20));

        assert_eq!(stats.count, 3);
        assert_eq!(stats.total, Duration::from_millis(60));
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(30));
        assert_eq!(stats.avg(), Duration::from_millis(20));
    }

    #[test]
    fn fmt_duration_formats_microseconds() {
        let d = Duration::from_micros(500);
        assert_eq!(fmt_duration(d), "500\u{00B5}s");
    }

    #[test]
    fn fmt_duration_formats_milliseconds() {
        let d = Duration::from_micros(1500);
        assert_eq!(fmt_duration(d), "1.50ms");
    }

    #[test]
    fn fmt_duration_formats_seconds() {
        let d = Duration::from_millis(1500);
        assert_eq!(fmt_duration(d), "1.50s");
    }

    #[test]
    fn fmt_duration_formats_minutes() {
        let d = Duration::from_secs(120);
        assert_eq!(fmt_duration(d), "2.00m");
    }

    #[test]
    fn display_no_samples() {
        let stats = RuntimeStats::default();
        assert_eq!(stats.to_string(), "no samples");
    }

    #[test]
    fn display_with_samples() {
        let mut stats = RuntimeStats::default();

        stats.update(Duration::from_millis(10));
        stats.update(Duration::from_millis(20));
        stats.update(Duration::from_millis(30));

        let s = stats.to_string();

        assert!(s.contains("min="));
        assert!(s.contains("avg="));
        assert!(s.contains("max="));
    }

    #[test]
    fn calc_entropy_small_case() {
        // m: masks {A0->{0}, A1->{0}, A2->{1}}
        let m = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(1),
            Bitset::from_word(1),
            Bitset::from_word(2),
        ]));
        // left_poss: p1=[0,0,1], p2=[0,1,1], p3=[1,0,1], p4=[1,1,1]
        let p1 = MaskedMatching::from_matching_ref(&[vec![0], vec![0], vec![1]]);
        let p2 = MaskedMatching::from_matching_ref(&[vec![0], vec![1], vec![1]]);
        let p3 = MaskedMatching::from_matching_ref(&[vec![1], vec![0], vec![1]]);
        let p4 = MaskedMatching::from_matching_ref(&[vec![1], vec![1], vec![1]]);
        let left = vec![p1, p2, p3, p4];
        let h = calc_entropy(&m, &left);
        // expected distribution: l=3 (1), l=2 (2), l=1 (1) -> probs 0.25,0.5,0.25 -> entropy 1.5
        let expected = 1.5;
        let diff = (h - expected).abs();
        assert!(diff < 1e-9, "entropy mismatch: {} vs {}", h, expected);
    }

    #[test]
    fn calc_entropy_empty_left_poss() {
        let m = MaskedMatching::from_masks(SmallVec::from_slice(&[]));
        let left: Vec<MaskedMatching> = vec![];
        let h = calc_entropy(&m, &left);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn calc_entropy_identical_left_poss() {
        let m = MaskedMatching::from_masks(SmallVec::from_slice(&[Bitset::from_word(1)]));
        let p = MaskedMatching::from_masks(SmallVec::from_slice(&[Bitset::from_word(1)]));
        let left = vec![p.clone(), p.clone(), p];
        let h = calc_entropy(&m, &left);
        // All l = 1, so single bucket -> entropy = 0
        assert_eq!(h, 0.0);
    }

    #[test]
    fn calc_entropy_varied_case() {
        let m = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(1),
            Bitset::from_word(2),
        ]));
        let p1 = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(1),
            Bitset::from_word(2),
        ])); // -> 2 lights
        let p2 = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(1),
            Bitset::from_word(0),
        ])); // -> 1 light
        let left = vec![p1, p2];
        let h = calc_entropy(&m, &left);
        assert_eq!(h, 1.0);
    }
}
