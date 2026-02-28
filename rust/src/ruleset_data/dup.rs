//! This module implements a dup_data which tracks how often people occur in multi-matches
//! (dup/trip). Here it is mandatory that only one multi-match exists.

use std::collections::HashMap;
use std::io;
use std::io::Write;

use anyhow::{Context, Result};

use crate::ruleset_data::utils::{
    aggregate_by_bitset, aggregate_by_individual_a, aggregate_by_individual_b, print_stats,
};
use crate::ruleset_data::RuleSetData;
use crate::Lut;
use crate::{matching_repr::bitset::Bitset, matching_repr::MaskedMatching, ruleset::RuleSet};

/// Collect statistics about "dup" (or "trip") events.
///
/// Internally keeps a map from `(index_in_set_a, bitset_of_b_indices)` -> count.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DupData {
    // key: (index_in_set_a, bitset_of_b_indices)
    // value: count
    cnt: HashMap<(usize, Bitset), usize>,
}

impl RuleSetData for DupData {
    fn push(&mut self, m: &MaskedMatching) -> Result<()> {
        let k = m
            .iter()
            .enumerate()
            // for "dup" rule, ther is always just one dup => find is the right function
            // returns an Option
            .find(|(_, j)| j.count() > 1)
            .with_context(|| "something went wrong")?;
        *self.cnt.entry(k).or_default() += 1;
        Ok(())
    }

    fn print(
        &self,
        full: bool,
        ruleset: &RuleSet,
        map_a: &[String],
        map_b: &[String],
        _lut_b: &Lut,
        total: u128,
    ) -> Result<()> {
        let word = match ruleset {
            RuleSet::XTimesDup(_) => "Dup",
            RuleSet::SomeoneIsTrip | RuleSet::FixedTrip(_) => "Trip",
            _ => unreachable!(),
        };

        let stats = DupStats::new(self);
        stats.fmt(&mut io::stdout(), full, map_a, map_b, total, word)?;
        Ok(())
    }
}

struct DupStats {
    full_matches: Vec<((usize, Bitset), usize)>,
    by_bitset: Vec<(Bitset, usize)>,
    by_individual: Vec<(IdBase, usize)>,
    by_a: Vec<(usize, usize)>,
}

impl DupStats {
    fn new(data: &DupData) -> Self {
        let mut full_matches = data.cnt.clone().into_iter().collect::<Vec<_>>();
        full_matches.sort_by(|(a, a_cnt), (b, b_cnt)| b_cnt.cmp(a_cnt).then_with(|| a.cmp(b)));

        let by_bitset = aggregate_by_bitset(&data.cnt);

        let by_individual = aggregate_by_individual_b(&data.cnt);

        let by_a = aggregate_by_individual_a(&data.cnt);

        Self {
            full_matches,
            by_bitset,
            by_individual,
            by_a,
        }
    }

    fn fmt<W: Write>(
        self,
        mut f: &mut W,
        full: bool,
        map_a: &[String],
        map_b: &[String],
        total: u128,
        word: &'static str,
    ) -> std::io::Result<()> {
        print_stats(
            &mut f,
            &format!("Pr[{word}]"),
            total,
            self.full_matches,
            |&(a_idx, bs)| {
                format!(
                    "{} -> {:?}",
                    map_a[a_idx],
                    bs.iter()
                        .map(|b| map_b[b as usize].clone())
                        .collect::<Vec<_>>()
                )
            },
            full,
            4,
        )?;

        print_stats(
            &mut f,
            &format!("Pr[{word}]"),
            total,
            self.by_bitset,
            |bs| {
                format!(
                    "{:?}",
                    bs.iter()
                        .map(|i| map_b[i as usize].clone())
                        .collect::<Vec<_>>()
                )
            },
            full,
            5,
        )?;

        print_stats(
            &mut f,
            &format!("Pr[{word}]"),
            total,
            self.by_individual,
            |&b| map_b[b as usize].clone(),
            full,
            5,
        )?;

        print_stats(
            &mut f,
            &format!("Pr[{word}]"),
            total,
            self.by_a,
            |&a_idx| map_a[a_idx].clone(),
            full,
            5,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn push_simple() -> Result<()> {
        let mm = MaskedMatching::from_matching_ref(&[vec![1, 2], vec![3]]);
        let mut dup = DupData::default();

        dup.push(&mm)?;
        assert_eq!(
            dup.cnt,
            HashMap::from_iter([((0, Bitset::from_idxs(&[1, 2])), 1),])
        );

        dup.push(&mm)?;
        assert_eq!(
            dup.cnt,
            HashMap::from_iter([((0, Bitset::from_idxs(&[1, 2])), 2),])
        );

        Ok(())
    }

    #[test]
    fn new_simple() -> Result<()> {
        let dup = DupData {
            cnt: HashMap::from_iter([
                ((0, Bitset::from_idxs(&[1, 2])), 3),
                ((1, Bitset::from_idxs(&[2])), 5),
                ((2, Bitset::from_idxs(&[1])), 1),
            ]),
        };
        let stats = DupStats::new(&dup);
        assert_eq!(
            stats.full_matches,
            vec![
                ((1, Bitset::from_idxs(&[2])), 5),
                ((0, Bitset::from_idxs(&[1, 2])), 3),
                ((2, Bitset::from_idxs(&[1])), 1),
            ]
        );

        let mut exp_bitset = vec![
            (Bitset::from_idxs(&[2]), 5),
            (Bitset::from_idxs(&[1, 2]), 3),
            (Bitset::from_idxs(&[1]), 1),
        ];
        exp_bitset.sort_by(|a, b| b.1.cmp(&a.1));
        assert_eq!(stats.by_bitset, exp_bitset);

        let mut exp_individual = vec![(2, 8), (1, 4)];
        exp_individual.sort_by(|a, b| b.1.cmp(&a.1));
        assert_eq!(stats.by_individual, exp_individual);

        let mut exp_by_a = vec![(1, 5), (0, 3), (2, 1)];
        exp_by_a.sort_by(|a, b| b.1.cmp(&a.1));
        assert_eq!(stats.by_a, exp_by_a);

        Ok(())
    }

    #[test]
    fn fmt_full_output() -> Result<()> {
        let mut dup = DupData::default();
        dup.cnt.insert((0, Bitset::from_idxs(&[1, 2])), 7);

        let stats = DupStats::new(&dup);

        let mut buf = Vec::new();
        stats.fmt(
            &mut buf,
            true,
            &["A".into()],
            &["a".into(), "b".into(), "c".into()],
            7,
            "Dup",
        )?;
        let out = String::from_utf8(buf)?;

        let expected = r#"Pr[Dup]: 
  100.0%/7: A -> ["b", "c"]
Pr[Dup]: 
  100.0%/7: ["b", "c"]
Pr[Dup]: 
  100.0%/7: c
  100.0%/7: b
Pr[Dup]: 
  100.0%/7: A
"#;
        assert_eq!(out, expected);
        Ok(())
    }

    #[test]
    fn fmt_truncated_output() -> Result<()> {
        let mut dup = DupData::default();
        for i in 0..10 {
            dup.cnt.insert((i, Bitset::from_idxs(&[i as u8 % 5])), 1);
        }

        let stats = DupStats::new(&dup);

        let a_labels: Vec<String> = (0..10).map(|i| format!("A{i}")).collect();
        let b_labels: Vec<String> = (0..5).map(|i| format!("B{i}")).collect();

        let mut buf = Vec::new();
        stats.fmt(&mut buf, false, &a_labels, &b_labels, 10, "Dup")?;
        let out = String::from_utf8(buf)?;

        let expected = r#"top4 Pr[Dup]: 10.0%/1: A0 -> ["B0"] | 10.0%/1: A1 -> ["B1"] | 10.0%/1: A2 -> ["B2"] | 10.0%/1: A3 -> ["B3"]
top5 Pr[Dup]: 20.0%/2: ["B4"] | 20.0%/2: ["B3"] | 20.0%/2: ["B2"] | 20.0%/2: ["B1"] | 20.0%/2: ["B0"]
top5 Pr[Dup]: 20.0%/2: B4 | 20.0%/2: B3 | 20.0%/2: B2 | 20.0%/2: B1 | 20.0%/2: B0
top5 Pr[Dup]: 10.0%/1: A0 | 10.0%/1: A1 | 10.0%/1: A2 | 10.0%/1: A3 | 10.0%/1: A4
"#;

        assert_eq!(out, expected);
        Ok(())
    }
}
