// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements a dup_data which tracks how often people occur in multi-matches
//! (dup/trip). This module also works for `RuleSetDupX` where multiple duplicates can be present
//! (but only duplicates, no triples)

use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::io::Write;

use anyhow::{Context, Result};

use crate::matching_repr::IdBase;
use crate::matching_repr::{bitset::Bitset, MaskedMatching};
use crate::ruleset::RuleSet;
use crate::ruleset::RuleSetDupX;
use crate::ruleset_data::utils::{
    aggregate_by_bitset, aggregate_by_individual_a, aggregate_by_individual_b, print_stats,
};
use crate::ruleset_data::RuleSetData;
use crate::Lut;

/// DupXData collects counts for duplicate/trip patterns for the "dup_x" ruleset.
#[derive(Debug, Clone, PartialEq)]
pub struct DupXData {
    /// the counts aggregated during the simulation
    ///
    /// key: (index_in_set_a, bitset_of_b_indices)
    /// value: count
    cnt: HashMap<(usize, Bitset), usize>,
    /// data from the ruleset needed for the evaluation
    rs: RuleSetDupX,
}

impl DupXData {
    /// Construct new DupXData from the ruleset parameters.
    pub fn new(rs: RuleSetDupX) -> Result<Self> {
        Ok(Self {
            cnt: HashMap::default(),
            rs: rs.clone(),
        })
    }

    /// Print a single block of information (split off for easier readability)
    ///
    /// - `first` bool to achieve proper *join*ing of the strings/outputs
    /// - `full` whether to truncate the output to x elements
    /// - `ruleset` the ruleset on which the printing is based on
    /// - `map_a`/`map_b` maps idx_a/idx_b to names
    /// - `total` the total amount of possible solutions left
    /// - `hdr` a header to be printed later when displaying the results
    /// - `query` filter data to entries which all contain this item
    /// - `query_not` filter data to entries which all do not contain any of these items
    #[allow(clippy::too_many_arguments)]
    fn print_one(
        &self,
        first: bool,
        full: bool,
        ruleset: &RuleSet,
        map_a: &[String],
        map_b: &[String],
        total: u128,
        query: Option<IdBase>,
        query_not: &HashSet<IdBase>,
        hdr: &str,
    ) -> Result<()> {
        let word = match ruleset {
            RuleSet::XTimesDup(_) => "Dup",
            RuleSet::SomeoneIsTrip | RuleSet::FixedTrip(_) => "Trip",
            _ => unreachable!(),
        };

        let mut w = io::stdout();
        let stats = DupXStats::new(self, hdr, query, query_not);
        stats.fmt(&mut w, full, map_a, map_b, total, word)?;
        if !first {
            writeln!(w, ".")?;
        }
        Ok(())
    }
}

impl RuleSetData for DupXData {
    fn push(&mut self, m: &MaskedMatching) -> Result<()> {
        for k in m.iter().enumerate().filter(|(_, j)| j.count() > 1) {
            *self.cnt.entry(k).or_default() += 1;
        }
        Ok(())
    }

    fn print(
        &self,
        full: bool,
        ruleset: &RuleSet,
        map_a: &[String],
        map_b: &[String],
        lut_b: &Lut,
        total: u128,
    ) -> Result<()> {
        let mut first = true;
        for d in self.rs.1.iter() {
            let not = dup_query_not(&self.rs, lut_b, d)?;
            let q = dup_query(lut_b, d)?;
            self.print_one(
                first,
                full,
                ruleset,
                map_a,
                map_b,
                total,
                Some(q),
                &not,
                &format!("Pr[]s for dup with {d}"),
            )?;
            first = false;
        }

        if self.rs.0 > 0 {
            let not = self
                .rs
                .1
                .iter()
                .map(|i| {
                    lut_b
                        .get(i)
                        .map(|x| *x as IdBase)
                        .with_context(|| format!("{i} not found"))
                })
                .collect::<Result<HashSet<_>>>()?;
            self.print_one(
                first,
                full,
                ruleset,
                map_a,
                map_b,
                total,
                None,
                &not,
                "Pr[]s for unknown dup",
            )?;
            // first = false;
        }
        Ok(())
    }
}

/// A struct collecting the results after the evaluation step
/// Can be printed/displayed via `fmt`
struct DupXStats<'a> {
    /// a header printed when displaying this data
    hdr: &'a str,

    /// stats/counts grouped by the matches (regarding the dups)
    full_matches: Vec<((usize, Bitset), usize)>,
    /// stats/counts grouped by the combination of individuals from set_b
    by_bitset: Vec<(Bitset, usize)>,
    /// stats/counts grouped by the individual in set_b
    by_individual: Vec<(IdBase, usize)>,
    /// stats/counts grouped by the individual in set_a
    by_a: Vec<(usize, usize)>,
}

impl<'a> DupXStats<'a> {
    /// Create a new DupXStats from DupXData `data` - performs already the evaluation step
    ///
    /// - `hdr` a header to be printed later when displaying the results
    /// - `query` filter data to entries which all contain this item
    /// - `query_not` filter data to entries which all do not contain any of these items
    fn new(data: &DupXData, hdr: &'a str, query: Option<IdBase>, query_not: &HashSet<IdBase>) -> Self {
        // filter according to query / query_not
        let filtered = match query {
            Some(q) => data
                .cnt
                .clone()
                .into_iter()
                .filter(|((_, bs), _)| {
                    bs.contains_idx(q) && bs.iter().all(|j| !query_not.contains(&j))
                })
                .collect::<HashMap<_, _>>(),
            None => data
                .cnt
                .clone()
                .into_iter()
                .filter(|((_, bs), _)| bs.iter().all(|j| !query_not.contains(&j)))
                .collect::<HashMap<_, _>>(),
        };

        let mut full_matches = Vec::from_iter(filtered.clone());
        full_matches.sort_by(|(a, a_cnt), (b, b_cnt)| b_cnt.cmp(a_cnt).then_with(|| b.cmp(a)));

        let by_bitset = aggregate_by_bitset(&filtered);

        let by_individual = aggregate_by_individual_b(&filtered);
        // println!("   / 200 %"); // TODO: dup_**X** => not always 200

        let by_a = aggregate_by_individual_a(&filtered);

        Self {
            hdr,
            full_matches,
            by_bitset,
            by_individual,
            by_a,
        }
    }

    /// Writer inspired by the Display trait but with additional perameters
    ///
    /// Writes the evaluated stats to `f`
    ///
    /// - `full` whether to truncate the lists to x elements
    /// - `map_a`/`map_b` maps idx_a/idx_b to names
    /// - `total` total amount of possible solutions left
    /// - `word` something like an identifier of what type of information is printed here
    fn fmt<W: Write>(
        self,
        mut f: &mut W,
        full: bool,
        map_a: &[String],
        map_b: &[String],
        total: u128,
        word: &'static str,
    ) -> std::io::Result<()> {
        writeln!(f, "{}", self.hdr)?;
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
            &format!("Pr({word})"),
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
            &format!("Pr({word})"),
            total,
            self.by_individual,
            |&b| map_b[b as usize].clone(),
            full,
            5,
        )?;

        print_stats(
            &mut f,
            &format!("Pr{word}"),
            total,
            self.by_a,
            |&a_idx| map_a[a_idx].clone(),
            full,
            5,
        )
    }
}

/// build a (negative/)not-query which excludes everything except `d`
fn dup_query_not(rs: &RuleSetDupX, lut_b: &Lut, d: &String) -> Result<HashSet<IdBase>> {
    rs.1.iter()
        .filter_map(|i| {
            if i != d {
                Some(
                    lut_b
                        .get(i)
                        .map(|x| *x as IdBase)
                        .with_context(|| format!("{i} not found")),
                )
            } else {
                None
            }
        })
        .collect::<Result<HashSet<_>>>()
}

/// build a (positive) query which searches for `d`
fn dup_query(lut_b: &Lut, d: &String) -> Result<IdBase> {
    lut_b
        .get(d)
        .map(|d| *d as IdBase)
        .with_context(|| format!("{d} not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn push_simple() -> Result<()> {
        let mm = MaskedMatching::from_matching_ref(&[vec![1, 2], vec![3]]);
        let mut dupx = DupXData::new(RuleSetDupX::default())?;

        dupx.push(&mm)?;
        assert_eq!(
            dupx.cnt,
            HashMap::from_iter([((0, Bitset::from_idxs(&[1, 2])), 1)])
        );

        dupx.push(&mm)?;
        assert_eq!(
            dupx.cnt,
            HashMap::from_iter([((0, Bitset::from_idxs(&[1, 2])), 2)])
        );

        Ok(())
    }

    #[test]
    fn new_simple() -> Result<()> {
        let dupx = DupXData {
            cnt: HashMap::from_iter([
                ((0, Bitset::from_idxs(&[1, 2])), 3),
                ((1, Bitset::from_idxs(&[2])), 5),
                ((2, Bitset::from_idxs(&[1])), 1),
            ]),
            rs: RuleSetDupX::default(),
        };

        let stats = DupXStats::new(&dupx, "hdr", None, &HashSet::new());
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
        let mut dupx = DupXData::new(RuleSetDupX::default())?;
        dupx.cnt.insert((0, Bitset::from_idxs(&[1, 2])), 7);

        let stats = DupXStats::new(&dupx, "Header", None, &HashSet::new());

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

        let expected = r#"Header
Pr[Dup]: 
  100.0%/7: A -> ["b", "c"]
Pr(Dup): 
  100.0%/7: ["b", "c"]
Pr(Dup): 
  100.0%/7: c
  100.0%/7: b
PrDup: 
  100.0%/7: A
"#;
        assert_eq!(out, expected);
        Ok(())
    }

    #[test]
    fn fmt_truncated_output() -> Result<()> {
        let mut dupx = DupXData::new(RuleSetDupX::default())?;
        for i in 0..10 {
            dupx.cnt.insert((i, Bitset::from_idxs(&[i as u8 % 5])), 1);
        }

        let stats = DupXStats::new(&dupx, "Hdr", None, &HashSet::new());

        let a_labels: Vec<String> = (0..10).map(|i| format!("A{i}")).collect();
        let b_labels: Vec<String> = (0..5).map(|i| format!("B{i}")).collect();

        let mut buf = Vec::new();
        stats.fmt(&mut buf, false, &a_labels, &b_labels, 10, "Dup")?;
        let out = String::from_utf8(buf)?;

        let expected = r#"Hdr
top4 Pr[Dup]: 10.0%/1: A9 -> ["B4"] | 10.0%/1: A8 -> ["B3"] | 10.0%/1: A7 -> ["B2"] | 10.0%/1: A6 -> ["B1"]
top5 Pr(Dup): 20.0%/2: ["B4"] | 20.0%/2: ["B3"] | 20.0%/2: ["B2"] | 20.0%/2: ["B1"] | 20.0%/2: ["B0"]
top5 Pr(Dup): 20.0%/2: B4 | 20.0%/2: B3 | 20.0%/2: B2 | 20.0%/2: B1 | 20.0%/2: B0
top5 PrDup: 10.0%/1: A0 | 10.0%/1: A1 | 10.0%/1: A2 | 10.0%/1: A3 | 10.0%/1: A4
"#;

        assert_eq!(out, expected);
        Ok(())
    }

    #[test]
    fn dup_query_not_simple() -> Result<()> {
        let rs = RuleSetDupX::default();
        let lut = HashMap::from_iter([
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ]);
        let not = dup_query_not(&rs, &lut, &"a".to_string())?;
        assert!(not.is_empty());
        Ok(())
    }

    #[test]
    fn dup_query_simple() -> Result<()> {
        let lut = HashMap::from_iter([("a".to_string(), 42)]);
        let q = dup_query(&lut, &"a".to_string())?;
        assert_eq!(q, 42);
        Ok(())
    }
}
