/*
sim_ayto
Copyright (C) 2025  Lukas Heindl

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

pub mod parse;
mod generators;
mod utils;

use crate::matching_repr::MaskedMatching;
use crate::ruleset::generators::{add_dup, add_trip, someone_is_dup, someone_is_trip};
use anyhow::{Context, Result};
use core::iter::zip;
use permutator::{Combination, Permutation};
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::iterstate::IterStateTrait;
use crate::Lut;

pub type RuleSetDupX = (usize, Vec<String>);
#[derive(Debug, Clone, Default)]
pub enum RuleSet {
    XTimesDup(RuleSetDupX),
    SomeoneIsTrip,
    NToN,
    FixedTrip(String),
    #[default]
    Eq,
}

impl RuleSet {
    // TODO: would be nicer to not have to convert Vec<Vec<u8>> to MaskedMatching
    // instead directly construct MaskedMatching (avoids allocations)
    pub fn iter_perms<T: IterStateTrait>(
        &self,
        lut_a: &Lut,
        lut_b: &Lut,
        is: &mut T,
        output: bool,
        cache: &Option<PathBuf>,
    ) -> Result<()> {
        if output {
            is.start();
        }
        if let Some(c) = cache {
            let file = File::open(c)?;
            let reader = BufReader::new(file);
            for (i, line) in reader.lines().enumerate() {
                let p = serde_json::from_str::<MaskedMatching>(&line?)?;
                is.step(i, p, output)?;
            }
        } else {
            match self {
                RuleSet::Eq => {
                    for (i, p) in (0..lut_a.len() as u8)
                        .map(|i| vec![i])
                        .collect::<Vec<_>>()
                        .permutation()
                        .enumerate()
                    {
                        is.step(i, p.into(), output)?;
                    }
                }
                RuleSet::XTimesDup((unkown_cnt, fixed)) => {
                    let fixed_num = fixed.iter().map(|d| lut_b[d] as u8).collect::<Vec<_>>();
                    let mut x = (0..lut_b.len() as u8)
                        .filter(|i| !fixed_num.contains(i))
                        .map(|i| vec![i])
                        .collect::<Vec<_>>();

                    let iter = x.permutation();

                    let iter = someone_is_dup(iter, *unkown_cnt);

                    let first_iter: Box<dyn Iterator<Item = Vec<Vec<u8>>>> = Box::new(iter);
                    let final_iter = fixed_num
                        .into_iter()
                        .fold(first_iter, |iter, add| Box::new(add_dup(iter, add)));

                    for (i, p) in final_iter.into_iter().enumerate() {
                        is.step(i, p.into(), output)?;
                    }
                }
                RuleSet::SomeoneIsTrip => {
                    let mut x = (0..lut_b.len() as u8).map(|i| vec![i]).collect::<Vec<_>>();
                    let x = x.permutation();
                    for (i, p) in someone_is_trip(x).enumerate() {
                        is.step(i, p.into(), output)?;
                    }
                }
                RuleSet::FixedTrip(s) => {
                    let mut x = (0..lut_b.len() as u8)
                        .filter(|i| *i != (*lut_b.get(s).unwrap() as u8))
                        .map(|i| vec![i])
                        .collect::<Vec<_>>();
                    let x = x.permutation();
                    for (i, p) in add_trip(
                        x,
                        *lut_b
                            .get(s)
                            .with_context(|| format!("Invalid index {}", s))?
                            as u8,
                    )
                    .enumerate()
                    {
                        is.step(i, p.into(), output)?;
                    }
                }
                RuleSet::NToN => {
                    let len = lut_a.len() / 2;
                    let mut i = 0_usize;
                    for ks in (0..lut_a.len() as u8).collect::<Vec<_>>().combination(len) {
                        let mut vs = (0..lut_a.len() as u8)
                            .filter(|x| !ks.contains(&x))
                            .collect::<Vec<_>>();
                        for p in vs.permutation().filter_map(|x| {
                            let mut c = vec![vec![]; lut_a.len()];
                            for (k, v) in zip(ks.clone(), x) {
                                if k <= &v {
                                    return None;
                                }
                                c[*k as usize] = vec![v];
                            }
                            Some(c)
                        }) {
                            is.step(i, p.into(), output)?;
                            i += 1;
                        }
                    }
                }
            }
        }
        if output {
            is.finish();
        }
        Ok(())
    }

    pub fn get_perms_amount(
        &self,
        size_map_a: usize,
        size_map_b: usize,
        cache: &Option<PathBuf>,
    ) -> Result<usize> {
        if let Some(c) = cache {
            let file = File::open(c)?;
            let reader = BufReader::new(file);
            let line_count = reader.lines().count();
            return Ok(line_count);
        }
        Ok(match self {
            RuleSet::XTimesDup((unkown_cnt, fixed)) => {
                // number of buckets / each permutation
                let a = size_map_a;
                // number of "items" to distribute
                let b = size_map_b;
                // number of "items" which must be placed in a double bucket
                let f = fixed.len();
                // number of additional "items" which are placed double buckets
                let s = unkown_cnt;

                // function foo(a,b,s,f) return (math.factorial(a)*math.factorial(b-f)*math.factorial(2*s+2*f))/(math.factorial(s+f)*math.factorial(a-s-f)*math.factorial(b-a+s)*2^(s+f)) end

                // choose which buckets should be double-buckets
                //   => choose (s+f) positions out of a positions
                let f_a = permutator::divide_factorial(a, a - (s + f));
                // choose which "items" to place in the single-buckets
                //   => choose a-(s+f) items from b-f available items
                //   (simplified the denominator)
                let f_b = permutator::divide_factorial(b - f, b - (a - s));
                // "items" left to distribute: 2l = b-(a-s-f)
                //   -> make l pairs out of them
                //   -> order all items (1), then remove duplicates (just swapped) (2), then ignore oder of pairs (3)
                //   => (2l)! / 2^l / l!
                //   -> assign pairs to double-bucket position
                //   => l!
                let f_c = permutator::divide_factorial(b - (a - s - f), s + f)
                    / (2_usize).pow((s + f) as u32);
                f_a * f_b * f_c
            }
            // choose one of setA to have the triple (a) and distribute the remaining ones (b!/3!)
            RuleSet::SomeoneIsTrip => size_map_a * permutator::factorial(size_map_b) / 6,
            // chose one of setA to have the triple (a) and distribute the remaining ones without
            // the fixed one ((b-1)!/2!)
            RuleSet::FixedTrip(_) => size_map_a * permutator::factorial(size_map_b - 1) / 2,
            RuleSet::Eq => permutator::factorial(size_map_a),
            // first choose the items for the first set, then distribute the rest. Avoid double
            // counting. binom(X,2X) * X! / 2
            RuleSet::NToN => {
                permutator::divide_factorial(size_map_a, size_map_a / 2) / (1 << (size_map_a / 2))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::collections::HashSet;

    #[derive(Default)]
    struct TestingIterState {
        seen: Vec<MaskedMatching>,
    }
    impl IterStateTrait for TestingIterState {
        fn start(&mut self) {}
        fn finish(&mut self) {}

        fn step(&mut self, _i: usize, p: MaskedMatching, _output: bool) -> Result<()> {
            self.seen.push(p);
            Ok(())
        }
    }

    #[test]
    fn test_iter_perms_eq() {
        let ground_truth: HashSet<Vec<Vec<u8>>> =
            HashSet::from([vec![vec![0], vec![1]], vec![vec![1], vec![0]]]);
        let eq_rule = RuleSet::Eq;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let mut is = TestingIterState::default();

        eq_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            let x = x
                .iter()
                .map(|y| {
                    let mut y = y.clone();
                    y.sort();
                    y
                })
                .collect::<Vec<_>>();
            assert!(
                ground_truth.contains(&x),
                "generated {:?} which is not in ground truth",
                x
            );
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_someone_is_dup() {
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![1, 2], vec![0]],
            vec![vec![0], vec![1, 2]],
            vec![vec![0, 1], vec![2]],
            vec![vec![1], vec![0, 2]],
            vec![vec![0, 2], vec![1]],
            vec![vec![2], vec![0, 1]],
        ]);
        let dup_rule = RuleSet::XTimesDup((1, vec![]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0), ("B", 1), ("C", 2)].map(|(k, v)| (k.to_string(), v)));
        let mut is = TestingIterState::default();

        dup_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            let x = x
                .iter()
                .map(|y| {
                    let mut y = y.clone();
                    y.sort();
                    y
                })
                .collect::<Vec<_>>();
            assert!(
                ground_truth.contains(&x),
                "generated {:?} which is not in ground truth",
                x
            );
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_someone_is_dup2() {
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![0, 1], vec![2, 3]],
            vec![vec![0, 2], vec![1, 3]],
            vec![vec![0, 3], vec![1, 2]],
            vec![vec![1, 2], vec![0, 3]],
            vec![vec![1, 3], vec![0, 2]],
            vec![vec![2, 3], vec![0, 1]],
        ]);
        let dup_rule = RuleSet::XTimesDup((2, vec![]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("A", 0), ("B", 1), ("C", 2), ("D", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        let mut is = TestingIterState::default();

        dup_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            let x = x
                .iter()
                .map(|y| {
                    let mut y = y.clone();
                    y.sort();
                    y
                })
                .collect::<Vec<_>>();
            assert!(
                ground_truth.contains(&x),
                "generated {:?} which is not in ground truth",
                x
            );
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_someone_is_trip() {
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![1, 2, 3], vec![0]],
            vec![vec![1], vec![0, 2, 3]],
            vec![vec![0, 2, 3], vec![1]],
            vec![vec![0], vec![1, 2, 3]],
            vec![vec![0, 1, 3], vec![2]],
            vec![vec![2], vec![0, 1, 3]],
            vec![vec![3], vec![0, 1, 2]],
            vec![vec![0, 1, 2], vec![3]],
        ]);
        let trip_rule = RuleSet::SomeoneIsTrip;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("A", 0), ("B", 1), ("C", 2), ("D", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        let mut is = TestingIterState::default();

        trip_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            let x = x
                .iter()
                .map(|y| {
                    let mut y = y.clone();
                    y.sort();
                    y
                })
                .collect::<Vec<_>>();
            assert!(
                ground_truth.contains(&x),
                "generated {:?} which is not in ground truth",
                x
            );
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_fixed_dup() {
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![0, 2], vec![1]],
            vec![vec![0], vec![1, 2]],
            vec![vec![1, 2], vec![0]],
            vec![vec![1], vec![0, 2]],
        ]);
        let dup_rule = RuleSet::XTimesDup((0, vec!["C".to_string()]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0), ("B", 1), ("C", 2)].map(|(k, v)| (k.to_string(), v)));
        let mut is = TestingIterState::default();

        dup_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            let x = x
                .iter()
                .map(|y| {
                    let mut y = y.clone();
                    y.sort();
                    y
                })
                .collect::<Vec<_>>();
            assert!(
                ground_truth.contains(&x),
                "generated {:?} which is not in ground truth",
                x
            );
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_fixed_trip() {
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![1, 2, 3], vec![0]],
            vec![vec![0], vec![1, 2, 3]],
            vec![vec![0, 1, 3], vec![2]],
            vec![vec![1], vec![0, 2, 3]],
            vec![vec![0, 2, 3], vec![1]],
            vec![vec![2], vec![0, 1, 3]],
        ]);
        let trip_rule = RuleSet::FixedTrip("D".to_string());
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("A", 0), ("B", 1), ("C", 2), ("D", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        let mut is = TestingIterState::default();

        trip_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            let x = x
                .iter()
                .map(|y| {
                    let mut y = y.clone();
                    y.sort();
                    y
                })
                .collect::<Vec<_>>();
            assert!(
                ground_truth.contains(&x),
                "generated {:?} which is not in ground truth",
                x
            );
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_xdup() {
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![0, 4], vec![1, 3], vec![2]],
            vec![vec![0, 4], vec![1], vec![2, 3]],
            vec![vec![0, 3], vec![1, 4], vec![2]],
            vec![vec![0], vec![1, 4], vec![2, 3]],
            vec![vec![0, 3], vec![1], vec![2, 4]],
            vec![vec![0], vec![1, 3], vec![2, 4]],
            vec![vec![1, 4], vec![0, 3], vec![2]],
            vec![vec![1, 4], vec![0], vec![2, 3]],
            vec![vec![1, 3], vec![0, 4], vec![2]],
            vec![vec![1], vec![0, 4], vec![2, 3]],
            vec![vec![1, 3], vec![0], vec![2, 4]],
            vec![vec![1], vec![0, 3], vec![2, 4]],
            vec![vec![2, 4], vec![0, 3], vec![1]],
            vec![vec![2, 4], vec![0], vec![1, 3]],
            vec![vec![2, 3], vec![0, 4], vec![1]],
            vec![vec![2], vec![0, 4], vec![1, 3]],
            vec![vec![2, 3], vec![0], vec![1, 4]],
            vec![vec![2], vec![0, 3], vec![1, 4]],
            vec![vec![0, 4], vec![2, 3], vec![1]],
            vec![vec![0, 4], vec![2], vec![1, 3]],
            vec![vec![0, 3], vec![2, 4], vec![1]],
            vec![vec![0], vec![2, 4], vec![1, 3]],
            vec![vec![0, 3], vec![2], vec![1, 4]],
            vec![vec![0], vec![2, 3], vec![1, 4]],
            vec![vec![1, 4], vec![2, 3], vec![0]],
            vec![vec![1, 4], vec![2], vec![0, 3]],
            vec![vec![1, 3], vec![2, 4], vec![0]],
            vec![vec![1], vec![2, 4], vec![0, 3]],
            vec![vec![1, 3], vec![2], vec![0, 4]],
            vec![vec![1], vec![2, 3], vec![0, 4]],
            vec![vec![2, 4], vec![1, 3], vec![0]],
            vec![vec![2, 4], vec![1], vec![0, 3]],
            vec![vec![2, 3], vec![1, 4], vec![0]],
            vec![vec![2], vec![1, 4], vec![0, 3]],
            vec![vec![2, 3], vec![1], vec![0, 4]],
            vec![vec![2], vec![1, 3], vec![0, 4]],
            vec![vec![3, 4], vec![1, 2], vec![0]],
            vec![vec![4], vec![1, 2], vec![0, 3]],
            vec![vec![3, 4], vec![1], vec![0, 2]],
            vec![vec![4], vec![1, 3], vec![0, 2]],
            vec![vec![1, 2], vec![3, 4], vec![0]],
            vec![vec![1, 2], vec![4], vec![0, 3]],
            vec![vec![1, 3], vec![4], vec![0, 2]],
            vec![vec![1], vec![3, 4], vec![0, 2]],
            vec![vec![0, 2], vec![3, 4], vec![1]],
            vec![vec![0, 2], vec![4], vec![1, 3]],
            vec![vec![0, 3], vec![4], vec![1, 2]],
            vec![vec![0], vec![3, 4], vec![1, 2]],
            vec![vec![3, 4], vec![0, 2], vec![1]],
            vec![vec![4], vec![0, 2], vec![1, 3]],
            vec![vec![3, 4], vec![0], vec![1, 2]],
            vec![vec![4], vec![0, 3], vec![1, 2]],
            vec![vec![1, 2], vec![0, 3], vec![4]],
            vec![vec![1, 2], vec![0], vec![3, 4]],
            vec![vec![1, 3], vec![0, 2], vec![4]],
            vec![vec![1], vec![0, 2], vec![3, 4]],
            vec![vec![0, 2], vec![1, 3], vec![4]],
            vec![vec![0, 2], vec![1], vec![3, 4]],
            vec![vec![0, 3], vec![1, 2], vec![4]],
            vec![vec![0], vec![1, 2], vec![3, 4]],
            vec![vec![0, 1], vec![2, 3], vec![4]],
            vec![vec![0, 1], vec![2], vec![3, 4]],
            vec![vec![2, 3], vec![0, 1], vec![4]],
            vec![vec![2], vec![0, 1], vec![3, 4]],
            vec![vec![3, 4], vec![0, 1], vec![2]],
            vec![vec![4], vec![0, 1], vec![2, 3]],
            vec![vec![0, 1], vec![3, 4], vec![2]],
            vec![vec![0, 1], vec![4], vec![2, 3]],
            vec![vec![2, 3], vec![4], vec![0, 1]],
            vec![vec![2], vec![3, 4], vec![0, 1]],
            vec![vec![3, 4], vec![2], vec![0, 1]],
            vec![vec![4], vec![2, 3], vec![0, 1]],
        ]);
        let rule = RuleSet::XTimesDup((1, vec!["D".to_string()]));
        let lut_a = HashMap::from([("A", 0), ("B", 1), ("C", 2)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("A", 0), ("B", 1), ("C", 2), ("D", 3), ("E", 4)].map(|(k, v)| (k.to_string(), v)),
        );
        let mut is = TestingIterState::default();

        rule.iter_perms(&lut_a, &lut_b, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            let x = x
                .iter()
                .map(|y| {
                    let mut y = y.clone();
                    y.sort();
                    y
                })
                .collect::<Vec<_>>();
            assert!(
                ground_truth.contains(&x),
                "generated {:?} which is not in ground truth",
                x
            );
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_nn() {
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![], vec![0], vec![], vec![], vec![2], vec![3]],
            vec![vec![], vec![0], vec![], vec![], vec![3], vec![2]],
            vec![vec![], vec![0], vec![], vec![2], vec![], vec![4]],
            vec![vec![], vec![], vec![0], vec![1], vec![], vec![4]],
            vec![vec![], vec![], vec![0], vec![], vec![1], vec![3]],
            vec![vec![], vec![], vec![0], vec![], vec![3], vec![1]],
            vec![vec![], vec![], vec![1], vec![0], vec![], vec![4]],
            vec![vec![], vec![], vec![1], vec![], vec![0], vec![3]],
            vec![vec![], vec![], vec![1], vec![], vec![3], vec![0]],
            vec![vec![], vec![], vec![], vec![0], vec![1], vec![2]],
            vec![vec![], vec![], vec![], vec![0], vec![2], vec![1]],
            vec![vec![], vec![], vec![], vec![1], vec![0], vec![2]],
            vec![vec![], vec![], vec![], vec![1], vec![2], vec![0]],
            vec![vec![], vec![], vec![], vec![2], vec![0], vec![1]],
            vec![vec![], vec![], vec![], vec![2], vec![1], vec![0]],
        ]);
        let nn_rule = RuleSet::NToN;
        let lut = HashMap::from(
            [("A", 0), ("B", 1), ("C", 2), ("D", 3), ("E", 4), ("F", 5)]
                .map(|(k, v)| (k.to_string(), v)),
        );
        let mut is = TestingIterState::default();

        nn_rule
            .iter_perms(&lut, &lut, &mut is, false, &None)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is
            .seen
            .iter()
            .map(|i| TryInto::<Vec<Vec<u8>>>::try_into(i).unwrap())
        {
            assert!(ground_truth.contains(&x));
        }
        // check if the lengths fit
        assert_eq!(is.seen.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.seen.len(),
            is.seen.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_get_perms_amout() {
        let rs = RuleSet::Eq;
        assert_eq!(rs.get_perms_amount(1, 1, &None).unwrap(), 1);
        assert_eq!(rs.get_perms_amount(2, 2, &None).unwrap(), 2);
        assert_eq!(rs.get_perms_amount(3, 3, &None).unwrap(), 6);

        let rs = RuleSet::XTimesDup((1, vec![]));
        assert_eq!(rs.get_perms_amount(1, 2, &None).unwrap(), 1);
        assert_eq!(rs.get_perms_amount(2, 3, &None).unwrap(), 6);
        assert_eq!(rs.get_perms_amount(3, 4, &None).unwrap(), 36);

        let rs = RuleSet::XTimesDup((0, vec!["A".to_string()]));
        assert_eq!(rs.get_perms_amount(1, 2, &None).unwrap(), 1);
        assert_eq!(rs.get_perms_amount(2, 3, &None).unwrap(), 4);
        assert_eq!(rs.get_perms_amount(3, 4, &None).unwrap(), 18);

        let rs = RuleSet::XTimesDup((1, vec!["A".to_string()]));
        assert_eq!(rs.get_perms_amount(2, 4, &None).unwrap(), 6);
        assert_eq!(rs.get_perms_amount(3, 5, &None).unwrap(), 72);

        let rs = RuleSet::SomeoneIsTrip;
        assert_eq!(rs.get_perms_amount(1, 3, &None).unwrap(), 1);
        assert_eq!(rs.get_perms_amount(2, 4, &None).unwrap(), 8);
        assert_eq!(rs.get_perms_amount(3, 5, &None).unwrap(), 60);

        let rs = RuleSet::FixedTrip("A".to_string());
        assert_eq!(rs.get_perms_amount(1, 3, &None).unwrap(), 1);
        assert_eq!(rs.get_perms_amount(2, 4, &None).unwrap(), 6);
        assert_eq!(rs.get_perms_amount(3, 5, &None).unwrap(), 36);

        let rs = RuleSet::NToN;
        assert_eq!(rs.get_perms_amount(3, 3, &None).unwrap(), 3);
        assert_eq!(rs.get_perms_amount(4, 4, &None).unwrap(), 3);
        assert_eq!(rs.get_perms_amount(5, 5, &None).unwrap(), 15);
    }
}
