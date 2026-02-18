use std::collections::HashMap;

use ayto::matching_repr::MaskedMatching;
use chrono::{DateTime, TimeZone, Utc};
use indicatif::ProgressBar;

/// Entropy calculation for a candidate `m` across `left_poss`.
///
/// This is a small pure function and unit tested below.
pub(crate) fn calc_entropy(m: &MaskedMatching, left_poss: &[MaskedMatching]) -> f64 {
    let total = left_poss.len() as f64;

    let mut lights = [0u32; 11];
    for p in left_poss {
        // assume:
        // - p is the solution
        // - m is how they sit in the night
        let l = m.calculate_lights(p);
        lights[l as usize] += 1;
    }

    let x = lights
        .into_iter()
        .filter(|&i| i > 0)
        .map(|i| {
            let p = (i as f64) / total;
            -p * p.log2()
        })
        .sum();
    println!("{x}");
    x
}

/// Format a millisecond timestamp into HH:MM:SS for the progress display.
fn format_time(ms: u128) -> String {
    let secs = (ms / 1000) as i64;
    let nsecs = ((ms % 1000) * 1_000_000) as u32;

    let dt: DateTime<Utc> = Utc.timestamp_opt(secs, nsecs).unwrap();
    dt.format("%H:%M:%S").to_string()
}

pub(crate) fn set_pb_msg(pb: &ProgressBar, active: &HashMap<usize, u128>) {
    pb.set_message(format!(
        "active:{} {}",
        active.len(),
        active
            .iter()
            .map(|(id, start)| format!("#{}@{}", id, format_time(*start)))
            .collect::<Vec<_>>()
            .join(", ")
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ayto::matching_repr::bitset::Bitset;
    use ayto::matching_repr::MaskedMatching;

    // -----------------------------
    // calc_entropy
    // -----------------------------

    #[test]
    fn calc_entropy_small_case() {
        // m: masks {A0->{0}, A1->{0}, A2->{1}}
        let m = MaskedMatching::from_masks(vec![
            Bitset::from_word(1),
            Bitset::from_word(1),
            Bitset::from_word(2),
        ]);
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
        let m = MaskedMatching::from_masks(vec![]);
        let left: Vec<MaskedMatching> = vec![];
        let h = calc_entropy(&m, &left);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn calc_entropy_identical_left_poss() {
        let m = MaskedMatching::from_masks(vec![Bitset::from_word(1)]);
        let p = MaskedMatching::from_masks(vec![Bitset::from_word(1)]);
        let left = vec![p.clone(), p.clone(), p];
        let h = calc_entropy(&m, &left);
        // All l = 1, so single bucket -> entropy = 0
        assert_eq!(h, 0.0);
    }

    #[test]
    fn calc_entropy_varied_case() {
        let m = MaskedMatching::from_masks(vec![
            Bitset::from_word(1),
            Bitset::from_word(2),
        ]);
        let p1 = MaskedMatching::from_masks(vec![
            Bitset::from_word(1),
            Bitset::from_word(2),
        ]);
        let p2 = MaskedMatching::from_masks(vec![
            Bitset::from_word(1),
            Bitset::from_word(0),
        ]);
        let left = vec![p1, p2];
        let h = calc_entropy(&m, &left);
        assert!(h > 0.0);
    }

    // -----------------------------
    // format_time
    // -----------------------------

    #[test]
    fn format_time_known_values() {
        assert_eq!(format_time(0), "00:00:00");
        assert_eq!(format_time(1_000), "00:00:01");
        assert_eq!(format_time(61_000), "00:01:01");
        assert_eq!(format_time(3_600_000), "01:00:00");
        assert_eq!(format_time(3_661_000), "01:01:01");
    }
}
