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

use crate::ruleset_data::dummy::DummyData;
use crate::ruleset_data::dup::DupData;
use crate::ruleset_data::dup_x::DupXData;
use crate::ruleset_data::RuleSetData;
use anyhow::{ensure, Context, Result};
use core::iter::zip;
use permutator::{Combination, Permutation};
use serde::Deserialize;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{game::IterState, Lut};

fn add_dup<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    add: u8,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        (0..perm.len()).filter_map({
            let perm = perm.clone();
            move |idx| {
                // only add the duplicate if it becomes a duplicate (not a triple)
                if perm[idx].len() > 1 {
                    return None;
                }
                let mut c = perm.clone();
                c[idx].push(add);
                Some(c)
            }
        })
    })
}

fn add_trip<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    add: u8,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        // select who has the dup
        (0..perm.len() - 1).filter_map(move |idx| {
            // only add the duplicate if it becomes a duplicate (not a triple)
            if perm[idx].len() > 1 {
                return None;
            }
            // only count once regardless the ordering
            if perm[idx][0] < perm[perm.len() - 1][0] {
                return None;
            }
            // the element at perm[len-1] is the dup => add it
            let mut c = perm.clone();
            let x = c.pop()?;
            c[idx].push(x[0]);
            c[idx].push(add);
            Some(c)
        })
    })
}

fn someone_is_dup<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    cnt: usize,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        let split = perm.len() - cnt;

        let recipients = perm[..split].to_vec();
        let dups = perm[split..].iter().map(|v| v[0]).collect::<Vec<_>>();

        let mut outputs: Vec<Vec<Vec<u8>>> = Vec::new();

        // internal values used for backtracking
        let mut cur = Vec::<usize>::with_capacity(cnt);

        // recursive DFS defined as local function (doesn't capture environment)
        fn dfs(
            recipients: &Vec<Vec<u8>>,
            dups: &Vec<u8>,
            // indices already containing a duplicate
            // used: &mut Vec<bool>,
            start: usize,
            // maps dups-idx to at which index to insert the duplicate
            cur: &mut Vec<usize>,
            outputs: &mut Vec<Vec<Vec<u8>>>,
        ) {
            if cur.len() == dups.len() {
                // build resulting c from base and cur mapping
                let mut c = recipients.clone();
                for (j, &rec_idx) in cur.iter().enumerate() {
                    if c[rec_idx][0] > dups[j] {
                        return;
                    }
                    c[rec_idx].push(dups[j]);
                }
                outputs.push(c);
                return;
            }
            // select/search index where to insert duplicate
            for i in start..recipients.len() {
                cur.push(i);
                dfs(recipients, dups, i + 1, cur, outputs);
                cur.pop();
            }
        }

        dfs(&recipients, &dups, 0, &mut cur, &mut outputs);
        outputs.into_iter()
    })
}

fn someone_is_trip<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        // if perm[perm.len() - 1][0] < perm[perm.len() - 2][0] {
        //     return;
        // }
        // select who has the trip
        (0..perm.len() - 2).filter_map(move |idx| {
            // only count once regardless the ordering
            if !(perm[idx][0] < perm[perm.len() - 1][0]
                && perm[perm.len() - 1][0] < perm[perm.len() - 2][0])
            {
                return None;
            }
            // the element at perm[len-2],perm[len-1] are the trip => add them
            let mut c = perm.clone();
            let x = c.pop()?;
            c[idx].push(x[0]);
            let x = c.pop()?;
            c[idx].push(x[0]);
            Some(c)
        })
    })
}

#[derive(Deserialize, Debug)]
pub enum RuleSetParse {
    SomeoneIsTrip,
    XTimesDup(Vec<Option<String>>),
    NToN,
    FixedTrip(String),
    Eq,
}

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
impl RuleSetParse {
    pub fn finalize_parsing(self) -> RuleSet {
        match self {
            RuleSetParse::SomeoneIsTrip => RuleSet::SomeoneIsTrip,
            RuleSetParse::NToN => RuleSet::NToN,
            RuleSetParse::FixedTrip(s) => RuleSet::FixedTrip(s),
            RuleSetParse::Eq => RuleSet::Eq,
            RuleSetParse::XTimesDup(s) => {
                let nc = s.iter().filter(|s| s.is_none()).count();
                let ss = s.into_iter().flatten().collect::<Vec<_>>();
                RuleSet::XTimesDup((nc, ss))
            }
        }
    }
}

impl RuleSet {
    pub fn init_data(&self) -> Result<Box<dyn RuleSetData>> {
        Ok(match &self {
            RuleSet::SomeoneIsTrip => Box::new(DupData::default()),
            RuleSet::FixedTrip(_) => Box::new(DupData::default()),
            RuleSet::NToN => Box::new(DummyData::default()),
            RuleSet::Eq => Box::new(DummyData::default()),

            // RuleSet::XTimesDup(_, _) => Box::new(DupData::default()),
            RuleSet::XTimesDup(rs) => Box::new(DupXData::new(rs.clone())?),
        })
    }

    pub fn must_add_exclude(&self) -> bool {
        match &self {
            RuleSet::XTimesDup(_) | RuleSet::SomeoneIsTrip | RuleSet::FixedTrip(_) => true,
            RuleSet::Eq | RuleSet::NToN => false,
        }
    }

    pub fn constr_map_len(&self, a: usize, _b: usize) -> usize {
        match &self {
            RuleSet::XTimesDup(_)
            | RuleSet::SomeoneIsTrip
            | RuleSet::FixedTrip(_)
            | RuleSet::Eq => a,
            RuleSet::NToN => a / 2,
        }
    }

    pub fn must_sort_constraint(&self) -> bool {
        match &self {
            RuleSet::XTimesDup(_)
            | RuleSet::SomeoneIsTrip
            | RuleSet::FixedTrip(_)
            | RuleSet::Eq => false,
            RuleSet::NToN => true,
        }
    }

    pub fn validate_lut(&self, lut_a: &Lut, lut_b: &Lut) -> Result<()> {
        match self {
            RuleSet::XTimesDup((unkown_cnt, fixed)) => {
                let d = fixed.len() + unkown_cnt;
                ensure!(
                    lut_a.len() == lut_b.len() - d,
                    "length of setA ({}) and setB ({}) does not fit to XTimesDup (len: {}",
                    lut_a.len(),
                    lut_b.len(),
                    d
                );
                for d in fixed {
                    ensure!(
                        lut_b.contains_key(d),
                        "fixed dup ({}) is not contained in setB",
                        d
                    );
                }
            }
            RuleSet::SomeoneIsTrip => {
                ensure!(
                    lut_a.len() == lut_b.len() - 2,
                    "length of setA ({}) and setB ({}) does not fit to SomeoneIsTrip",
                    lut_a.len(),
                    lut_b.len()
                );
            }
            RuleSet::FixedTrip(s) => {
                ensure!(
                    lut_a.len() == lut_b.len() - 2,
                    "length of setA ({}) and setB ({}) does not fit to FixedTrip",
                    lut_a.len(),
                    lut_b.len()
                );
                ensure!(
                    lut_b.contains_key(s),
                    "fixed trip ({}) is not contained in setB",
                    s
                );
            }
            RuleSet::Eq => {
                ensure!(
                    lut_a.len() == lut_b.len(),
                    "length of setA ({}) and setB ({}) does not fit to Eq",
                    lut_a.len(),
                    lut_b.len()
                );
            }
            RuleSet::NToN => {
                ensure!(
                    lut_a.len() == lut_b.len(),
                    "length of setA ({}) and setB ({}) does not fit to NToN",
                    lut_a.len(),
                    lut_b.len()
                );
                ensure!(
                    lut_a == lut_b,
                    "with the n-to-n rule-set, both sets must be exactly the same"
                );
            }
        }
        Ok(())
    }

    pub fn ignore_pairing(&self, a: usize, b: usize) -> bool {
        match self {
            RuleSet::Eq
            | RuleSet::XTimesDup(_)
            | RuleSet::SomeoneIsTrip
            | RuleSet::FixedTrip(_) => false,
            RuleSet::NToN => a <= b,
        }
    }

    pub fn iter_perms(
        &self,
        lut_a: &Lut,
        lut_b: &Lut,
        is: &mut IterState,
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
                let p = serde_json::from_str::<Vec<Vec<u8>>>(&line?)?;
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
                        is.step(i, p, output)?;
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
                        is.step(i, p, output)?;
                    }
                }
                RuleSet::SomeoneIsTrip => {
                    let mut x = (0..lut_b.len() as u8).map(|i| vec![i]).collect::<Vec<_>>();
                    let x = x.permutation();
                    for (i, p) in someone_is_trip(x).enumerate() {
                        is.step(i, p, output)?;
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
                        is.step(i, p, output)?;
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
                            let mut c = vec![vec![u8::MAX]; lut_a.len()];
                            for (k, v) in zip(ks.clone(), x) {
                                if k <= &v {
                                    return None;
                                }
                                c[*k as usize] = vec![v];
                            }
                            Some(c)
                        }) {
                            is.step(i, p, output)?;
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

    #[test]
    fn test_validate_lut_nn() {
        let nn_rule = RuleSet::NToN;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        nn_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0)].map(|(k, v)| (k.to_string(), v)));
        assert!(nn_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(nn_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_eq() {
        let eq_rule = RuleSet::Eq;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        eq_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0)].map(|(k, v)| (k.to_string(), v)));
        assert!(eq_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_fixed_dup() {
        let dup_rule = RuleSet::XTimesDup((0, vec!["x".to_string()]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("x", 3)].map(|(k, v)| (k.to_string(), v)));
        dup_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(dup_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        assert!(dup_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_fixed_trip() {
        let trip_rule = RuleSet::FixedTrip("x".to_string());
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("x", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        trip_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(trip_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("d", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        assert!(trip_rule.validate_lut(&lut_a, &lut_b).is_err());
    }

    #[test]
    fn test_validate_lut_someone_is_dup() {
        let dup_rule = RuleSet::XTimesDup((1, vec![]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("x", 3)].map(|(k, v)| (k.to_string(), v)));
        dup_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(dup_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        dup_rule.validate_lut(&lut_a, &lut_b).unwrap();
    }

    #[test]
    fn test_validate_lut_soneone_is_trip() {
        let trip_rule = RuleSet::SomeoneIsTrip;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("x", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        trip_rule.validate_lut(&lut_a, &lut_b).unwrap();

        let lut_a = HashMap::from([("A", 0), ("B", 1), ("c", 2)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("a", 0), ("b", 1)].map(|(k, v)| (k.to_string(), v)));
        assert!(trip_rule.validate_lut(&lut_a, &lut_b).is_err());

        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from(
            [("a", 0), ("b", 1), ("c", 2), ("d", 3)].map(|(k, v)| (k.to_string(), v)),
        );
        trip_rule.validate_lut(&lut_a, &lut_b).unwrap();
    }

    #[test]
    fn test_iter_perms_eq() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
        let ground_truth: HashSet<Vec<Vec<u8>>> =
            HashSet::from([vec![vec![0], vec![1]], vec![vec![1], vec![0]]]);
        let eq_rule = RuleSet::Eq;
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        eq_rule.iter_perms(&lut_a, &lut_b, &mut is, false).unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is.left_poss {
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
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_someone_is_dup() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
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
        dup_rule.iter_perms(&lut_a, &lut_b, &mut is, false).unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is.left_poss {
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
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_someone_is_dup2() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
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
        dup_rule.iter_perms(&lut_a, &lut_b, &mut is, false).unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is.left_poss {
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
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_someone_is_trip() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
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
        trip_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is.left_poss {
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
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_fixed_dup() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
        let ground_truth: HashSet<Vec<Vec<u8>>> = HashSet::from([
            vec![vec![0, 2], vec![1]],
            vec![vec![0], vec![1, 2]],
            vec![vec![1, 2], vec![0]],
            vec![vec![1], vec![0, 2]],
        ]);
        let dup_rule = RuleSet::XTimesDup((0, vec!["C".to_string()]));
        let lut_a = HashMap::from([("A", 0), ("B", 1)].map(|(k, v)| (k.to_string(), v)));
        let lut_b = HashMap::from([("A", 0), ("B", 1), ("C", 2)].map(|(k, v)| (k.to_string(), v)));
        dup_rule.iter_perms(&lut_a, &lut_b, &mut is, false).unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is.left_poss {
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
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_fixed_trip() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
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
        trip_rule
            .iter_perms(&lut_a, &lut_b, &mut is, false)
            .unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is.left_poss {
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
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_xdup() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
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
        rule.iter_perms(&lut_a, &lut_b, &mut is, false).unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &mut is.left_poss {
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
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn test_iter_perms_nn() {
        let mut is = IterState::new(true, 0, vec![], &vec![]);
        let ground_truth: HashSet<Vec<u8>> = HashSet::from([
            vec![(255), (0), (255), (255), (2), (3)],
            vec![(255), (0), (255), (255), (3), (2)],
            vec![(255), (0), (255), (2), (255), (4)],
            vec![(255), (255), (0), (1), (255), (4)],
            vec![(255), (255), (0), (255), (1), (3)],
            vec![(255), (255), (0), (255), (3), (1)],
            vec![(255), (255), (1), (0), (255), (4)],
            vec![(255), (255), (1), (255), (0), (3)],
            vec![(255), (255), (1), (255), (3), (0)],
            vec![(255), (255), (255), (0), (1), (2)],
            vec![(255), (255), (255), (0), (2), (1)],
            vec![(255), (255), (255), (1), (0), (2)],
            vec![(255), (255), (255), (1), (2), (0)],
            vec![(255), (255), (255), (2), (0), (1)],
            vec![(255), (255), (255), (2), (1), (0)],
        ]);
        let nn_rule = RuleSet::NToN;
        let lut = HashMap::from(
            [("A", 0), ("B", 1), ("C", 2), ("D", 3), ("E", 4), ("F", 5)]
                .map(|(k, v)| (k.to_string(), v)),
        );
        nn_rule.iter_perms(&lut, &lut, &mut is, false).unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &is.left_poss {
            let x: Vec<_> = x.iter().map(|i| i[0]).collect();
            assert!(ground_truth.contains(&x));
        }
        // check if the lengths fit
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(
            is.left_poss.len(),
            is.left_poss.drain(..).collect::<HashSet<_>>().len()
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
