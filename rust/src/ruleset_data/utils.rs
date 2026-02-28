//! This module implements helpers used to evaluate the data gathered by ruleset_data.

use std::{collections::HashMap, io::Write, ops::AddAssign};

use crate::matching_repr::bitset::Bitset;

/// Helper that aggregates a `(usize, Bitset)` map by the Bitset itself.
/// cnt: (a, bs) -> increase count by v
/// returns a sorted vector
pub(super) fn aggregate_by_bitset<T>(cnt: &HashMap<(usize, Bitset), T>) -> Vec<(Bitset, T)>
where
    T: Copy + AddAssign + Default + Ord,
{
    let mut agg: HashMap<Bitset, T> = HashMap::new();
    for ((_, bs), v) in cnt.iter() {
        *agg.entry(*bs).or_default() += *v;
    }
    let mut vec: Vec<_> = agg.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    vec
}

/// Helper that aggregates by each individual index inside the Bitset.
/// cnt: (a, bs) -> increase count by v
/// returns a sorted vector
pub(super) fn aggregate_by_individual_b<T>(cnt: &HashMap<(usize, Bitset), T>) -> Vec<(u8, T)>
where
    T: Copy + AddAssign + Default + Ord,
{
    let mut agg: HashMap<u8, T> = HashMap::new();
    for ((_, bs), v) in cnt.iter() {
        for idx in bs.iter() {
            *agg.entry(idx).or_default() += *v;
        }
    }
    let mut vec: Vec<_> = agg.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    vec
}

/// Helper that aggregates by each individual index inside the Matching.
/// cnt: (a, bs) -> increase count by v
/// returns a sorted vector
pub(super) fn aggregate_by_individual_a<T>(cnt: &HashMap<(usize, Bitset), T>) -> Vec<(usize, T)>
where
    T: Copy + AddAssign + Default + Ord,
{
    let mut agg: HashMap<usize, T> = HashMap::new();
    for ((a, _), v) in cnt.iter() {
        *agg.entry(*a).or_default() += *v;
    }
    let mut vec: Vec<_> = agg.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    vec
}

/// Generic printer used by both modules.
pub(super) fn print_stats<T, W>(
    mut f: W,
    title: &str,
    total: u128,
    entries: impl IntoIterator<Item = (T, usize)>,
    fmt_key: impl Fn(&T) -> String,
    full: bool,
    top_n: usize,
) -> std::io::Result<()>
where
    T: std::fmt::Debug,
    W: Write,
{
    let mut vec: Vec<_> = entries.into_iter().collect();
    vec.sort_by(|(_, a_cnt), (_, b_cnt)| b_cnt.cmp(a_cnt));

    let iter: Box<dyn Iterator<Item = _>> = if full {
        write!(f, "{title}: ")?;
        Box::new(vec.into_iter())
    } else {
        write!(f, "top{top_n} {title}: ")?;
        Box::new(vec.into_iter().take(top_n))
    };

    let mut first = true;
    for (key, cnt) in iter {
        write!(
            f,
            "{}{}{:.1}%/{}: {}",
            if full { "\n  " } else { "" },
            if full {
                ""
            } else if !first {
                " | "
            } else {
                ""
            },
            (cnt as f64 / total as f64) * 100.0,
            cnt,
            fmt_key(&key)
        )?;
        first = false;
    }
    writeln!(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    #[test]
    fn aggregate_by_bitset_simple() -> Result<()> {
        let cnt = HashMap::from_iter([
            ((0, Bitset::from_idxs(&[1, 2])), 3),
            ((1, Bitset::from_idxs(&[2])), 5),
            ((2, Bitset::from_idxs(&[1])), 1),
            ((3, Bitset::from_idxs(&[1])), 1),
        ]);

        let res = aggregate_by_bitset(&cnt);

        let mut expected = vec![
            (Bitset::from_idxs(&[1, 2]), 3),
            (Bitset::from_idxs(&[2]), 5),
            (Bitset::from_idxs(&[1]), 2),
        ];
        expected.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));

        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn aggregate_by_individual_b_simple() -> Result<()> {
        let cnt = HashMap::from_iter([
            ((0, Bitset::from_idxs(&[1, 2])), 3),
            ((1, Bitset::from_idxs(&[2])), 5),
            ((2, Bitset::from_idxs(&[1])), 1),
        ]);

        let res = aggregate_by_individual_b(&cnt);

        let mut expected = vec![(2, 8), (1, 4)];
        expected.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));

        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn aggregate_by_individual_a_simple() -> Result<()> {
        let cnt = HashMap::from_iter([
            ((0, Bitset::from_idxs(&[1, 2])), 3),
            ((2, Bitset::from_idxs(&[2])), 5),
            ((2, Bitset::from_idxs(&[1])), 1),
        ]);

        let res = aggregate_by_individual_a(&cnt);

        let mut expected = vec![(0, 3), (2, 6)];
        expected.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        assert_eq!(res, expected);
        Ok(())
    }

    #[test]
    fn print_stats_full_simple() -> Result<()> {
        let entries = vec![(("key1".to_string(), 2), 5), (("key2".to_string(), 6), 10)];
        let mut buf = Vec::new();

        print_stats(
            &mut buf,
            "Title",
            15,
            entries.into_iter(),
            |k| k.0.clone(),
            true,
            4,
        )?;
        let out = String::from_utf8(buf)?;

        let expected = r#"Title: 
  66.7%/10: key2
  33.3%/5: key1
"#;

        assert_eq!(out, expected);
        Ok(())
    }

    #[test]
    fn print_stats_truncated_simple() -> Result<()> {
        let entries = (0..10)
            .map(|i| (format!("k{i}"), 1usize))
            .collect::<Vec<_>>();
        let mut buf = Vec::new();

        print_stats(
            &mut buf,
            "Title",
            10,
            entries.clone(),
            |k| k.clone(),
            false,
            5,
        )?;
        let out = String::from_utf8(buf)?;

        let expected = r#"top5 Title: 10.0%/1: k0 | 10.0%/1: k1 | 10.0%/1: k2 | 10.0%/1: k3 | 10.0%/1: k4
"#;

        assert_eq!(out, expected);

        Ok(())
    }
}
