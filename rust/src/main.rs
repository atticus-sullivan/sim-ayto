/*
sim_ayto
Copyright (C) 2024  Lukas Heindl

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

use anyhow::{ensure, Context, Result};
use clap::Parser;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL_CONDENSED, Cell, Color, Table,
};
use indicatif::{ProgressBar, ProgressStyle};
use permutator::{Combination, Permutation};
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Write;
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use std::{collections::HashMap, fs::File};

// TODO cleanup (multiple files?)
// TODO code review (try with chatGPT)

#[cfg(test)]
mod tests {
    use permutator::Permutation;

    use super::*;
    use std::collections::HashMap;

    fn constraint_def() -> Constraint {
        Constraint {
            exclude: None,
            exclude_s: None,
            no_exclude: false,
            map_s: HashMap::new(),
            check: CheckType::Lights(2),
            map: HashMap::from([(0, 1), (1, 2), (2, 0), (3, 3)]),
            eliminated: 0,
            eliminated_tab: vec![
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
                vec![0, 0, 0, 0, 0],
            ],
            entropy: None,
            left_after: None,
            hidden: false,
            r#type: ConstraintType::Night {
                num: 1.0,
                comment: String::from(""),
            },
        }
    }

    #[test]
    fn test_constraint_fits() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];
        assert!(!c.fits(&m).unwrap());
        match &mut c.check {
            CheckType::Eq => {}
            CheckType::Lights(l) => *l = 1,
        }
        assert!(c.fits(&m).unwrap());
    }

    #[test]
    fn test_constraint_eliminate() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];

        c.eliminate(&m).unwrap();
        assert_eq!(c.eliminated, 1);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![1, 0, 0, 0, 0],
                vec![0, 1, 0, 0, 0],
                vec![0, 0, 1, 0, 0],
                vec![0, 0, 0, 1, 1]
            ]
        );

        c.eliminate(&m).unwrap();
        assert_eq!(c.eliminated, 2);
        assert_eq!(
            c.eliminated_tab,
            vec![
                vec![2, 0, 0, 0, 0],
                vec![0, 2, 0, 0, 0],
                vec![0, 0, 2, 0, 0],
                vec![0, 0, 0, 2, 2]
            ]
        );
    }

    #[test]
    fn test_constraint_apply() {
        let mut c = constraint_def();
        let m: Matching = vec![vec![0], vec![1], vec![2], vec![3, 4]];

        c.eliminate(&m).unwrap();
        assert_eq!(c.eliminated, 1);

        let mut rem: Rem = (vec![vec![15; 5]; 4], 5 * 4 * 3 * 2 * 1 * 4 / 2);

        rem = c.apply_to_rem(rem).unwrap();
        assert_eq!(rem.1, 5 * 4 * 3 * 2 * 1 * 4 / 2 - 1);
        assert_eq!(
            rem.0,
            vec![
                vec![14, 15, 15, 15, 15],
                vec![15, 14, 15, 15, 15],
                vec![15, 15, 14, 15, 15],
                vec![15, 15, 15, 14, 14]
            ]
        );
    }

    #[test]
    fn test_someone_is_dup() {
        let mut x: Vec<Vec<u8>> = vec![vec![0],vec![1],vec![2]];
        let perm = x.permutation();
        let perm = Box::new(someone_is_dup(perm));

        let ground_truth = vec![
            vec![vec![2,1], vec![0]],
            vec![vec![0], vec![2,1]],
            vec![vec![1,0], vec![2]],
            vec![vec![1], vec![2,0]],
            vec![vec![2,0], vec![1]],
            vec![vec![2], vec![1,0]],
        ];
        let mut i = 0;
        for p in perm {
            assert_eq!(p, ground_truth[i]);
            i+=1;
        }
        assert_eq!(i, ground_truth.len());
    }

    #[test]
    fn test_someone_is_trip() {
        let mut x: Vec<Vec<u8>> = vec![vec![0],vec![1],vec![2], vec![3]];
        let perm = x.permutation();
        let perm = Box::new(someone_is_trip(perm));

        let ground_truth = vec![
            vec![vec![1,2,3], vec![0]],
            vec![vec![1], vec![0,2,3]],
            vec![vec![0,2,3], vec![1]],
            vec![vec![0], vec![1,2,3]],
            vec![vec![0,1,3], vec![2]],
            vec![vec![2], vec![0,1,3]],
            vec![vec![3], vec![0,1,2]],
            vec![vec![0,1,2], vec![3]],
        ];
        let mut i = 0;
        for p in perm {
            assert_eq!(p, ground_truth[i]);
            i+=1;
        }
        assert_eq!(i, ground_truth.len());
    }

    #[test]
    fn test_add_dup() {
        let mut x: Vec<Vec<u8>> = vec![vec![0],vec![1]];
        let perm = x.permutation();
        let perm = Box::new(add_dup(perm, 2));

        let ground_truth = vec![
            vec![vec![0,2], vec![1]],
            vec![vec![0], vec![1,2]],
            vec![vec![1,2], vec![0]],
            vec![vec![1], vec![0,2]],
        ];
        let mut i = 0;
        for p in perm {
            assert_eq!(p, ground_truth[i]);
            i+=1;
        }
        assert_eq!(i, ground_truth.len());
    }

    #[test]
    fn test_add_trip() {
        let mut x: Vec<Vec<u8>> = vec![vec![0],vec![1],vec![2]];
        let perm = x.permutation();
        let perm = Box::new(add_trip(perm, 3));

        let ground_truth = vec![
            vec![vec![2,1,3], vec![0]],
            vec![vec![0], vec![2,1,3]],
            vec![vec![1,0,3], vec![2]],
            vec![vec![1], vec![2,0,3]],
            vec![vec![2,0,3], vec![1]],
            vec![vec![2], vec![1,0,3]],
        ];
        let mut i = 0;
        for p in perm {
            assert_eq!(p, ground_truth[i]);
            i+=1;
        }
        assert_eq!(i, ground_truth.len());
    }

    #[test]
    fn test_iter_perms_nn() {
        let mut is = IterState::new(true, 15, vec![]);
        let ground_truth:HashSet<Vec<u8>> = HashSet::from([
            vec![(255), (0),   (255), (255), (2),   (3)],
            vec![(255), (0),   (255), (255), (3),   (2)],
            vec![(255), (0),   (255), (2),   (255), (4)],
            vec![(255), (255), (0),   (1),   (255), (4)],
            vec![(255), (255), (0),   (255), (1),   (3)],
            vec![(255), (255), (0),   (255), (3),   (1)],
            vec![(255), (255), (1),   (0),   (255), (4)],
            vec![(255), (255), (1),   (255), (0),   (3)],
            vec![(255), (255), (1),   (255), (3),   (0)],
            vec![(255), (255), (255), (0),   (1),   (2)],
            vec![(255), (255), (255), (0),   (2),   (1)],
            vec![(255), (255), (255), (1),   (0),   (2)],
            vec![(255), (255), (255), (1),   (2),   (0)],
            vec![(255), (255), (255), (2),   (0),   (1)],
            vec![(255), (255), (255), (2),   (1),   (0)],
        ]);
        let nn_rule = RuleSet::NToN;
        let lut = HashMap::from([("A", 0), ("B", 1), ("C", 2), ("D", 3), ("E", 4), ("F", 5)].map(|(k,v)| (k.to_string(), v)));
        nn_rule.iter_perms(&lut, &lut, &mut is).unwrap();

        // check if another permutation than from ground_truth was generated
        for x in &is.left_poss {
            let x:Vec<_> = x.iter().map(|i| i[0]).collect();
            assert!(ground_truth.contains(&x));
        }
        // check if the lengths fit
        assert_eq!(is.left_poss.len(), ground_truth.len());
        // check if duplicates were generated
        assert_eq!(is.left_poss.len(), is.left_poss.drain(..).collect::<HashSet<_>>().len());
    }
}

// TODO where to put this
fn add_dup<I: Iterator<Item = Vec<Vec<u8>>>>(
    vals: I,
    add: u8,
) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        (0..perm.len()).map(move |idx| {
            let mut c = perm.clone();
            c[idx].push(add);
            c
        })
    })
}

// TODO where to put this
fn add_trip<I: Iterator<Item = Vec<Vec<u8>>>>(vals: I, add: u8) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        // select who has the dup
        (0..perm.len() - 1).filter_map(move |idx| {
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

// TODO where to put this
fn someone_is_dup<I: Iterator<Item = Vec<Vec<u8>>>>(vals: I) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        // select who has the dup
        (0..perm.len() - 1).filter_map(move |idx| {
            // only count once regardless the ordering
            if perm[idx][0] < perm[perm.len() - 1][0] {
                return None;
            }
            // the element at perm[len-1] is the dup => add it
            let mut c = perm.clone();
            let x = c.pop()?;
            c[idx].push(x[0]);
            Some(c)
        })
    })
}

// TODO where to put this
fn someone_is_trip<I: Iterator<Item = Vec<Vec<u8>>>>(vals: I) -> impl Iterator<Item = Vec<Vec<u8>>> {
    vals.flat_map(move |perm| {
        // if perm[perm.len() - 1][0] < perm[perm.len() - 2][0] {
        //     return;
        // }
        // select who has the trip
        (0..perm.len() - 2).filter_map(move |idx| {
            // only count once regardless the ordering
            if !(perm[idx][0] < perm[perm.len() - 1][0] && perm[perm.len() - 1][0] < perm[perm.len() - 2][0]) {
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

#[derive(Deserialize, Debug, Clone)]
enum CheckType {
    Eq,
    Lights(u8)
}

#[derive(Deserialize, Debug, Clone)]
enum ConstraintType {
    Night { num: f32, comment: String },
    Box { num: f32, comment: String },
}

type Matching = Vec<Vec<u8>>;
type MapS = HashMap<String, String>;
type Map = HashMap<u8, u8>;
type Lut = HashMap<String, usize>;

type Rem = (Vec<Vec<u128>>, u128);

#[derive(Deserialize, Debug, Clone)]
struct Constraint {
    r#type: ConstraintType,
    #[serde(rename = "map")]
    map_s: MapS,
    check: CheckType,
    #[serde(default)]
    hidden: bool,
    #[serde(default, rename = "noExclude")]
    no_exclude: bool,
    #[serde(rename = "exclude")]
    exclude_s: Option<(String, Vec<String>)>,

    #[serde(skip)]
    map: Map,
    #[serde(skip)]
    exclude: Option<(u8, HashSet<u8>)>,
    #[serde(skip)]
    eliminated: u128,
    #[serde(skip)]
    eliminated_tab: Vec<Vec<u128>>,

    #[serde(skip)]
    entropy: Option<f64>,
    #[serde(skip)]
    left_after: Option<u128>,
}

impl Constraint {
    fn finalize_parsing(&mut self, lut_a: &Lut, lut_b: &Lut, map_len: usize) -> Result<()> {
        // check if map size is valid
        match self.r#type {
            ConstraintType::Night { .. } => {
                ensure!(self.map_s.len() == map_len, "Map in a night must contain exactly as many entries as set_a {} (was: {})", map_len, self.map_s.len());
                ensure!(self.exclude_s.is_none(), "Exclude is not yet supported for nights");
            }
            ConstraintType::Box { .. } => {
            }
        }

        self.eliminated_tab.reserve_exact(lut_a.len());
        for _ in 0..lut_a.len() {
            self.eliminated_tab.push(vec![0; lut_b.len()])
        }

        self.map.reserve(self.map_s.len());
        for (k, v) in &self.map_s {
            self.map.insert(
                *lut_a.get(k).with_context(|| format!("Invalid Key {}", k))? as u8,
                *lut_b.get(v).with_context(|| format!("Invalid Value {}", v))? as u8,
            );
        }

        if let Some(ex) = &self.exclude_s {
            let (ex_a,ex_b) = ex;
            let mut bs = HashSet::with_capacity(ex_b.len());
            let a = *lut_a.get(ex_a).with_context(|| format!("Invalid Key {}", ex_a))? as u8;
            for x in ex_b {
                bs.insert(*lut_b.get(x).with_context(|| format!("Invalid Value {}", x))? as u8);
            }
            self.exclude = Some((a, bs));
        }

        Ok(())
    }

    fn comment(&self) -> &str {
        match &self.r#type {
            ConstraintType::Night { num: _, comment } => &comment,
            ConstraintType::Box { num: _, comment } => &comment,
        }
    }

    fn type_str(&self) -> String {
        match &self.r#type {
            ConstraintType::Night { num, comment: _ } => format!("MN#{}", num),
            ConstraintType::Box { num, comment: _ } => format!("MB#{}", num),
        }
    }

    fn eliminate(&mut self, m: &Matching) -> Result<()> {
        for (i1, v) in m.iter().enumerate() {
            for &i2 in v {
                if i2 == 255 {
                    continue
                }
                self.eliminated_tab[i1][i2 as usize] += 1
            }
        }
        self.eliminated += 1;
        Ok(())
    }

    fn fits(&self, m: &Matching) -> Result<bool> {
        match &self.check {
            CheckType::Eq => {
                Ok(m.iter().enumerate().all(|(_,js)| {
                    self.map.iter().map(|(_,i2)| js.contains(i2)).fold(None, |acc, b| {
                        match acc {
                            Some(a) => Some(a == b),
                            None => Some(b),
                        }
                    }).unwrap()
                }))
            }
            CheckType::Lights(lights) => {
                let mut l = 0;
                for (i1, i2) in self.map.iter() {
                    if m[*i1 as usize].contains(i2) {
                        if l >= *lights {
                            return Ok(false);
                        }
                        l += 1;
                    }
                }
                if let Some(ex) = &self.exclude {
                    for i in &m[ex.0 as usize] {
                        if ex.1.contains(i) {
                            return Ok(false)
                        }
                    }
                }
                Ok(l == *lights)
            }
        }
    }

    fn merge(&mut self, other: &Self) -> Result<()> {
        self.eliminated += other.eliminated;
        ensure!(self.eliminated_tab.len() == other.eliminated_tab.len(), "eliminated_tab lengths do not match (self: {}, other: {})", self.eliminated_tab.len(), other.eliminated_tab.len());
        for (i, es) in self.eliminated_tab.iter_mut().enumerate() {
            ensure!(es.len() == other.eliminated_tab[i].len(), "eliminated_tab lengths do not match (self: {}, other: {})", es.len(), other.eliminated_tab[i].len());
            for (j, e) in es.iter_mut().enumerate() {
                *e += other.eliminated_tab[i][j];
            }
        }
        self.entropy = None;
        self.left_after = None;
        Ok(())
    }

    fn apply_to_rem(&mut self, mut rem: Rem) -> Option<Rem> {
        rem.1 -= self.eliminated;

        for (i, rs) in rem.0.iter_mut().enumerate() {
            for (j, r) in rs.iter_mut().enumerate() {
                *r -= self.eliminated_tab.get(i)?.get(j)?;
            }
        }

        self.left_after = Some(rem.1);

        let tmp = 1.0 - (self.eliminated as f64) / (rem.1 + self.eliminated) as f64;
        self.entropy = if tmp > 0.0 { Some(-tmp.log2()) } else { None };

        Some(rem)
    }

    fn stat_row(&self, map_a: &[String]) -> Vec<Cell> {
        let mut ret = vec![];
        match self.r#type {
            ConstraintType::Night { num, .. } => ret.push(Cell::new(format!("MN#{:02.1}", num))),
            ConstraintType::Box { num, .. } => ret.push(Cell::new(format!("MB#{:02.1}", num))),
        }
        match &self.check {
            CheckType::Eq => ret.push(Cell::new("E")),
            CheckType::Lights(lights) => ret.push(Cell::new(lights)),
        }
        ret.extend(
            map_a
                .iter()
                .map(|a| Cell::new(self.map_s.get(a).unwrap_or(&String::from("")))),
        );
        ret.push(Cell::new(String::from("")));
        ret.push(Cell::new(self.entropy.unwrap_or(std::f64::INFINITY)));

        ret
    }

    fn write_stats(&self, mbo: &mut File, mno: &mut File, info: &mut File) -> Result<()> {
        if self.hidden {
            return Ok(());
        }

        match self.r#type {
            ConstraintType::Night { num, .. } => {
                writeln!(
                    info,
                    "{} {}",
                    num * 2.0,
                    (self.left_after.context("total_left unset")? as f64).log2()
                )?;
                writeln!(
                    mno,
                    "{} {}",
                    num,
                    self.entropy.unwrap_or(std::f64::INFINITY)
                )?;
            }
            ConstraintType::Box { num, .. } => {
                writeln!(
                    info,
                    "{} {}",
                    num * 2.0 - 1.0,
                    (self.left_after.context("total_left unset")? as f64).log2()
                )?;
                writeln!(
                    mbo,
                    "{} {}",
                    num,
                    self.entropy.unwrap_or(std::f64::INFINITY)
                )?;
            }
        }
        Ok(())
    }

    fn print_hdr(&self) {
        match &self.check {
            CheckType::Eq => print!("Eq "),
            CheckType::Lights(l) => print!("{} ", l),
        }
        match &self.r#type {
            ConstraintType::Night { num, comment, .. } => print!("MN#{:02.1} {}", num, comment),
            ConstraintType::Box { num, comment, .. } => print!("MB#{:02.1} {}", num, comment),
        }
        println!();

        for (k, v) in &self.map_s {
            println!("{} -> {}", k, v);
        }

        println!("-> I = {}", self.entropy.unwrap_or(std::f64::INFINITY));
    }
}

#[derive(Deserialize, Debug)]
enum RuleSet {
    SomeoneIsDup,
    SomeoneIsTrip,
    NToN,
    FixedDup(String),
    FixedTrip(String),
    Eq,
}

impl std::default::Default for RuleSet {
    fn default() -> Self {
        RuleSet::Eq
    }
}

impl RuleSet {
    fn night_map_len(&self, a: usize, _b: usize) -> usize {
        if let RuleSet::NToN = self {
            a/2
        } else {
            a
        }
    }

    fn sorted_constraint(&self) -> bool {
        if let RuleSet::NToN = self {
            true
        } else {
            false
        }
    }

    fn validate_lut(&self, lut_a: &Lut, lut_b: &Lut) -> Result<()> {
        match self {
            RuleSet::SomeoneIsDup => {
                ensure!(lut_a.len() == lut_b.len()-1, "length of setA ({}) and setB ({}) does not fit to SomeoneIsDup", lut_a.len(), lut_b.len());
            },
            RuleSet::FixedDup(s) => {
                ensure!(lut_a.len() == lut_b.len()-1, "length of setA ({}) and setB ({}) does not fit to FixedDup", lut_a.len(), lut_b.len());
                ensure!(lut_b.contains_key(s), "fixed dup ({}) is not contained in setB", s);
            },
            RuleSet::SomeoneIsTrip => {
                ensure!(lut_a.len() == lut_b.len()-2, "length of setA ({}) and setB ({}) does not fit to SomeoneIsTrip", lut_a.len(), lut_b.len());
            },
            RuleSet::FixedTrip(s) => {
                ensure!(lut_a.len() == lut_b.len()-2, "length of setA ({}) and setB ({}) does not fit to FixedTrip", lut_a.len(), lut_b.len());
                ensure!(lut_b.contains_key(s), "fixed trip ({}) is not contained in setB", s);
            },
            RuleSet::Eq => {
                ensure!(lut_a.len() == lut_b.len(), "length of setA ({}) and setB ({}) does not fit to Eq", lut_a.len(), lut_b.len());
            },
            RuleSet::NToN => {
                ensure!(lut_a.len() == lut_b.len(), "length of setA ({}) and setB ({}) does not fit to NToN", lut_a.len(), lut_b.len());
                ensure!(lut_a == lut_b, "with the n-to-n rule-set, both sets must be exactly the same");
            },
        }
        Ok(())
    }

    fn ignore(&self, a: usize, b: usize) -> bool {
        match self {
            RuleSet::Eq | RuleSet::SomeoneIsDup | RuleSet::SomeoneIsTrip | RuleSet::FixedDup(_) | RuleSet::FixedTrip(_) => false,
            RuleSet::NToN => a <= b,
        }
    }

    fn iter_perms(&self, lut_a: &Lut, lut_b: &Lut, is: &mut IterState) -> Result<()> {
        is.progress.inc(0);
        match self {
            RuleSet::Eq => {
                for (i,p) in (0..lut_a.len() as u8).map(|i| vec![i]).collect::<Vec<_>>().permutation().enumerate() {
                    is.step(i, p)?;
                }
            },
            RuleSet::FixedDup(s) => {
                let mut x = (0..lut_b.len() as u8).filter(|i| *i != (*lut_b.get(s).unwrap() as u8)).map(|i| vec![i]).collect::<Vec<_>>();
                let x = x.permutation();
                for (i,p) in add_dup(x, *lut_b.get(s).with_context(|| format!("Invalid index {}", s))? as u8).enumerate() {
                    is.step(i, p)?;
                }
            },
            RuleSet::SomeoneIsDup => {
                let mut x = (0..lut_b.len() as u8).map(|i| vec![i]).collect::<Vec<_>>();
                let x = x.permutation();
                for (i,p) in someone_is_dup(x).enumerate() {
                    is.step(i, p)?;
                }
            },
            RuleSet::SomeoneIsTrip => {
                let mut x = (0..lut_b.len() as u8).map(|i| vec![i]).collect::<Vec<_>>();
                let x = x.permutation();
                for (i,p) in someone_is_trip(x).enumerate() {
                    is.step(i, p)?;
                }
            },
            RuleSet::FixedTrip(s) => {
                let mut x = (0..lut_b.len() as u8).filter(|i| *i != (*lut_b.get(s).unwrap() as u8)).map(|i| vec![i]).collect::<Vec<_>>();
                let x = x.permutation();
                for (i,p) in add_trip(x, *lut_b.get(s).with_context(|| format!("Invalid index {}", s))? as u8).enumerate() {
                    is.step(i, p)?;
                }
            },
            RuleSet::NToN => {
                let len = lut_a.len()/2;
                let mut i = 0 as usize;
                for ks in (0..lut_a.len() as u8).collect::<Vec<_>>().combination(len) {
                    let mut vs = (0..lut_a.len() as u8).filter(|x| !ks.contains(&x)).collect::<Vec<_>>();
                    for p in vs.permutation().filter_map(|x| {
                        let mut c = vec![vec![u8::MAX]; lut_a.len()];
                        for (k,v) in zip(ks.clone(), x) {
                            if k <= &v {
                                return None
                            }
                            c[*k as usize] = vec![v];
                        }
                        Some(c)
                    }) {
                        is.step(i,p)?;
                        i += 1;
                    }
                }
            },
        }
        is.progress.finish();
        Ok(())
    }

    fn get_perms_amount(&self, size_map_a: usize, size_map_b: usize) -> usize {
        match self {
            // choose one of setA to have the dups (a) and distribute the remaining ones (b!/2!)
            RuleSet::SomeoneIsDup => size_map_a * permutator::factorial(size_map_b) / 2,
            // choose one of setA to have the triple (a) and distribute the remaining ones (b!/3!)
            RuleSet::SomeoneIsTrip => size_map_a * permutator::factorial(size_map_b) / 6,
            RuleSet::FixedDup(_) => permutator::factorial(size_map_a) * size_map_a,
            // chose one of setA to have the triple (a) and distribute the remaining ones without
            // the fixed one ((b-1)!/2!)
            RuleSet::FixedTrip(_) => size_map_a * permutator::factorial(size_map_b-1) / 2,
            RuleSet::Eq => permutator::factorial(size_map_a),
            // first choose the items for the first set, then distribute the rest. Avoid double
            // counting. binom(X,2X) * X! / 2
            RuleSet::NToN => permutator::divide_factorial(size_map_a, size_map_a/2) / (1<<size_map_a/2),
        }
    }
}

struct IterState {
    constraints: Vec<Constraint>,
    tree_gen: bool,
    each: u128,
    total: u128,
    eliminated: u128,
    left_poss: Vec<Matching>,
    progress: ProgressBar,
    cnt_update: usize,
}

impl IterState {
    fn new(tree_gen: bool, perm_amount: usize, constraints: Vec<Constraint>) -> IterState {
        let is = IterState{
            constraints,
            tree_gen,
            each: 0,
            total: 0,
            eliminated: 0,
            left_poss: vec![],
            progress: ProgressBar::new(100),
            cnt_update: std::cmp::max(perm_amount / 50, 1),
        };
        is.progress.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar}] {pos:>3}/{len:3} (ETA: {eta})").unwrap().progress_chars("#>-"));
        is
    }

    fn step(&mut self, i: usize, p: Matching) -> Result<()> {
        // eprintln!("{:} {:?}", i, p);
        if i % self.cnt_update == 0 {
            self.progress.inc(2);
        }
        if p[1].contains(&0) {
            self.each += 1;
        }
        self.total += 1;
        let mut left = true;
        for c in &mut self.constraints {
            if !c.fits(&p)? {
                left = false;
                c.eliminate(&p)?;
                self.eliminated += 1;
                break;
            }
        }
        if left && self.tree_gen {
            self.left_poss.push(p);
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct Game {
    #[serde(rename = "constraints")]
    constraints_orig: Vec<Constraint>,
    rule_set: RuleSet,
    tree_gen: bool,
    tree_top: Option<String>,

    #[serde(rename = "setA")]
    map_a: Vec<String>,
    #[serde(rename = "setB")]
    map_b: Vec<String>,

    #[serde(skip)]
    dir: PathBuf,
    #[serde(skip)]
    stem: String,
    #[serde(skip)]
    lut_a: Lut,
    #[serde(skip)]
    lut_b: Lut,
}

impl Game {
    fn new_from_yaml(yaml_path: &Path, stem: &Path) -> Result<Game> {
        let mut g: Game = serde_yaml::from_reader(File::open(yaml_path)?)?;

        g.dir = stem
            .parent()
            .context("parent dir of stem not found")?
            .to_path_buf();
        g.stem = stem
            .file_stem()
            .context("No filename provided in stem")?
            .to_string_lossy()
            .into_owned();

        // build up the look up tables (LUT)
        for (lut, map) in [(&mut g.lut_a, &g.map_a), (&mut g.lut_b, &g.map_b)] {
            for (index, name) in map.iter().enumerate() {
                lut.insert(name.clone(), index);
            }
        }
        ensure!(g.lut_a.len() == g.map_a.len(), "something is wrong with the sets. There might be duplicates in setA (len: {}, dedup len: {}).", g.lut_a.len(), g.map_a.len());
        ensure!(g.lut_b.len() == g.map_b.len(), "something is wrong with the sets. There might be duplicates in setB (len: {}, dedup len: {}).", g.lut_b.len(), g.map_b.len());
        // validate the lut in combination with the ruleset
        g.rule_set.validate_lut(&g.lut_a, &g.lut_b)?;

        // postprocessing -> add exclude mapping list (with names)
        match &g.rule_set {
            RuleSet::SomeoneIsDup | RuleSet::SomeoneIsTrip | RuleSet::FixedDup(_) | RuleSet::FixedTrip(_) => {
                for c in &mut g.constraints_orig {
                    if c.no_exclude {continue}
                    if let CheckType::Lights(l) = c.check {
                        if !(l == 1 && c.map_s.len() == 1 && c.exclude_s.is_none()) {continue}
                        if let ConstraintType::Box { .. } = c.r#type {
                            for (k,v) in &c.map_s {
                                let bs: Vec<String> = g.map_b.iter()
                                    .filter(|&i| i != v)
                                    .map(|i| i.to_string())
                                    .collect();
                                c.exclude_s = Some((k.to_string(), bs));
                            }
                        }
                    }
                }
            },
            RuleSet::Eq | RuleSet::NToN => {},
        }

        // eg translates strings to indices (u8)
        for c in &mut g.constraints_orig {
            c.finalize_parsing(&g.lut_a, &g.lut_b, g.rule_set.night_map_len(g.lut_a.len(), g.lut_b.len()))?;
        }

        // postprocessing -> sort map if ruleset demands it
        if g.rule_set.sorted_constraint() {
            for c in &mut g.constraints_orig {
                c.map = c.map.drain().map(|(k, v)| {
                    if k < v {
                        (v, k)
                    } else {
                        (k, v)
                    }
                }).collect();
                c.map_s = c.map_s.drain().map(|(k, v)| {
                    if g.lut_a[&k] > g.lut_b[&v] {
                        (v, k)
                    } else {
                        (k, v)
                    }
                }).collect();
            }
        }

        Ok(g)
    }

    fn sim(&mut self) -> Result<()> {
        let perm_amount = self
            .rule_set
            .get_perms_amount(self.map_a.len(), self.map_b.len());

        let mut is = IterState::new(self.tree_gen, perm_amount, self.constraints_orig.clone());
        self.rule_set.iter_perms(&self.lut_a, &self.lut_b, &mut is)?;

        // fix is so that it can't be mutated anymore
        let is = &is;

        let mut rem: Rem = (vec![vec![is.each; self.map_b.len()]; self.map_a.len()], is.total);
        self.print_rem(&rem).context("Error printing")?;
        println!();

        let mut constr = vec![];
        let mut to_merge = vec![]; // collect hidden constraints to merge them down
        for c in &is.constraints {
            if c.hidden {
                to_merge.push(c);
            } else {
                let mut c = c.clone();
                // merge down previous hidden constraints
                while !to_merge.is_empty() {
                    c.merge(to_merge.pop().unwrap())?;
                }
                rem = c.apply_to_rem(rem).context("Apply to rem failed")?;
                c.print_hdr();
                self.print_rem(&rem).context("Error printing")?;
                constr.push(c);
                println!();
            }
        }

        if self.tree_gen {
            let dot_path = self.dir.join(self.stem.clone()).with_extension("dot");
            let ordering = self.tree_ordering(&is.left_poss);
            self.dot_tree(
                &is.left_poss,
                &ordering,
                &(constr[constr.len() - 1].type_str() + " / " + constr[constr.len() - 1].comment()),
                &mut File::create(dot_path.clone())?,
            )?;

            let pdf_path = dot_path.with_extension("pdf");
            Command::new("dot")
                .args([
                    "-Tpdf",
                    "-o",
                    pdf_path.to_str().context("pdf_path failed")?,
                    dot_path.to_str().context("dot_path failed")?,
                ])
                .output()
                .expect("dot command failed");

            let png_path = dot_path.with_extension("png");
            Command::new("dot")
                .args([
                    "-Tpng",
                    "-o",
                    png_path.to_str().context("png_path failed")?,
                    dot_path.to_str().context("dot_path failed")?,
                ])
                .output()
                .expect("dot command failed");
        }

        let dot_path = self
            .dir
            .join(self.stem.clone() + "_tab")
            .with_extension("dot");
        self.write_rem_dot(
            &rem,
            &(constr[constr.len() - 1].type_str() + " / " + constr[constr.len() - 1].comment()),
            &mut File::create(dot_path.clone())?,
        )?;

        let pdf_path = dot_path.with_extension("pdf");
        Command::new("dot")
            .args([
                "-Tpdf",
                "-o",
                pdf_path.to_str().context("pdf_path failed")?,
                dot_path.to_str().context("dot_path failed")?,
            ])
            .output()
            .expect("dot command failed");

        let png_path = dot_path.with_extension("png");
        Command::new("dot")
            .args([
                "-Tpng",
                "-o",
                png_path.to_str().context("png_path failed")?,
                dot_path.to_str().context("dot_path failed")?,
            ])
            .output()
            .expect("dot command failed");
        // println!("dir: {:?} dot_path: {:?} png_path: {:?} pdf_path: {:?}", self.dir, dot_path, png_path, pdf_path);

        self.do_statistics(&constr)?;

        println!(
            "Total permutations: {}  Permutations left: {}  Initial combinations for each pair: {}",
            is.total,
            is.total - is.eliminated,
            is.each
        );
        Ok(())
    }

    fn do_statistics(&self, merged_constraints: &Vec<Constraint>) -> Result<()> {
        let out_mb_path = self
            .dir
            .join(self.stem.clone() + "_statMB")
            .with_extension("out");
        let out_mn_path = self
            .dir
            .join(self.stem.clone() + "_statMN")
            .with_extension("out");
        let out_info_path = self
            .dir
            .join(self.stem.clone() + "_statInfo")
            .with_extension("out");

        let (mut mbo, mut mno, mut info) = (
            File::create(out_mb_path)?,
            File::create(out_mn_path)?,
            File::create(out_info_path)?,
        );
        for c in merged_constraints {
            c.write_stats(&mut mbo, &mut mno, &mut info)?;
        }

        let mut hdr = vec![
            Cell::new(""),
            Cell::new("L").set_alignment(comfy_table::CellAlignment::Center),
        ];
        hdr.extend(
            self.map_a
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );
        hdr.push(Cell::new("").set_alignment(comfy_table::CellAlignment::Center));
        hdr.push(Cell::new("I").set_alignment(comfy_table::CellAlignment::Center));

        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);

        for c in merged_constraints {
            table.add_row(c.stat_row(&self.map_a));
        }
        println!("{table}");

        Ok(())
    }

    fn write_rem_dot(&self, rem: &Rem, title: &str, writer: &mut File) -> Result<()> {
        writeln!(
            writer,
            "digraph structs {{ labelloc=\"b\"; label=\"Stand: {}\"; node[shape=plaintext] struct[label=<", title)?;
        writeln!(
            writer,
            "<table cellspacing=\"2\" border=\"0\" rows=\"*\" columns=\"*\">"
        )?;

        // header row
        writeln!(writer, "<tr>")?;
        writeln!(writer, "<td></td>")?; // first empty cell
        for h in &self.map_b {
            writeln!(writer, "<td><B>{h}</B></td>")?;
        }
        writeln!(writer, "</tr>")?;

        for (i, a) in self.map_a.iter().enumerate() {
            writeln!(writer, "<tr>")?;
            writeln!(writer, "<td><B>{a}</B></td>")?;

            let i = rem
                .0
                .get(i)
                .with_context(|| format!("Indexing rem with map failed (idx {})", i))?
                .iter()
                .map(|x| {
                    let val = (*x as f64) / (rem.1 as f64) * 100.0;
                    if 79.0 < val && val < 101.0 {
                        (val, "darkgreen")
                    } else if -1.0 < val && val < 1.0 {
                        (val, "red")
                    } else {
                        (val, "black")
                    }
                });
            for (v, font) in i {
                writeln!(
                    writer,
                    "<td><font color=\"{}\">{:03.4}</font></td>",
                    font, v
                )?;
            }
            writeln!(writer, "</tr>")?;
        }
        writeln!(writer, "</table>")?;
        writeln!(writer, ">];}}")?;

        Ok(())
    }

    fn print_rem(&self, rem: &Rem) -> Option<()> {
        let mut hdr = vec![Cell::new("")];
        hdr.extend(
            self.map_b
                .iter()
                .map(|x| Cell::new(x).set_alignment(comfy_table::CellAlignment::Center)),
        );
        let mut table = Table::new();
        table
            .force_no_tty()
            .enforce_styling()
            .load_preset(UTF8_FULL_CONDENSED)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(hdr);
        for (i, a) in self.map_a.iter().enumerate() {
            let i = rem.0.get(i)?.iter().enumerate().map(|(j,x)| {
                if self.rule_set.ignore(i,j) {
                    Cell::new("")
                } else {
                    let val = (*x as f64) / (rem.1 as f64) * 100.0;
                    if 79.0 < val && val < 101.0 {
                        Cell::new(format!("{:02.3}", val)).fg(Color::Green)
                    } else if -1.0 < val && val < 1.0 {
                        Cell::new(format!("{:02.3}", val)).fg(Color::Red)
                    } else {
                        Cell::new(format!("{:02.3}", val))
                    }
                }
            });
            let mut row = vec![Cell::new(a)];
            row.extend(i);
            table.add_row(row);
        }
        println!("{table}");
        println!("{} left -> {} bits left", rem.1, (rem.1 as f64).log2());
        Some(())
    }

    fn dot_tree(
        &self,
        data: &Vec<Matching>,
        ordering: &Vec<(usize, usize)>,
        title: &str,
        writer: &mut File,
    ) -> Result<()> {
        let mut nodes: HashSet<String> = HashSet::new();
        writeln!(
            writer,
            "digraph D {{ labelloc=\"b\"; label=\"Stand: {}\"; ranksep=0.8;",
            title
        )?;
        for p in data {
            let mut parent = String::from("root");
            for (i, _) in ordering {
                let mut node = parent.clone();
                node.push('/');
                node.push_str(
                    &p[*i]
                        .iter()
                        .map(|b| b.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );

                if nodes.insert(node.clone()) {
                    // if node is new
                    if p[*i].iter().filter(|&b| !self.rule_set.ignore(*i, *b as usize)).count() == 0 {
                        writeln!(writer, "\"{node}\"[label=\"\"]")?;
                    } else {
                        // only put content in that node if there is something meaning-full
                        // don't just skip the whole node since this would mess up the layering
                        writeln!(
                            writer,
                            "\"{node}\"[label=\"{}\"]",
                            self.map_a[*i].clone()
                            + "\\n"
                            + &p[*i]
                                .iter()
                                .filter(|&b| !self.rule_set.ignore(*i, *b as usize))
                                .map(|b| self.map_b[*b as usize].clone())
                                .collect::<Vec<_>>()
                                .join("\\n")
                        )?;
                    }
                    writeln!(writer, "\"{parent}\" -> \"{node}\";")?;
                }

                parent = node;
            }
        }
        writeln!(writer, "}}")?;
        Ok(())
    }

    fn tree_ordering(&self, data: &Vec<Matching>) -> Vec<(usize, usize)> {
        let mut tab = vec![HashSet::new(); self.map_a.len()];
        for p in data {
            for (i, js) in p.iter().enumerate() {
                if !self.rule_set.ignore(i, js[0] as usize) {
                    tab[i].insert(js);
                }
            }
        }

        let mut ordering: Vec<_> = tab.iter().enumerate().filter_map(|(i, x)| {
            if x.len() == 0 {
                None
            } else {
                Some((i, x.len()))
            }
        }).collect();
        match &self.tree_top {
            Some(ts) => {
                let t = self.lut_a[ts];
                ordering.sort_unstable_by_key(|(i, x)| {
                    // x values will always be positive, 1 will be the minimum / value for already
                    // fixed matches
                    // with (x-1)*2 we move that minimum to 0 and spread the values.
                    // In effect the value 1 will be unused. To sort the specified tree_top right
                    // below the already fixed matches this level is mapped to the value 1
                    // Why so complicated? To avoid using floats here, while still ensuring the
                    // order as specified.
                    if *i == t {
                        1
                    } else {
                        ((*x) - 1) * 2
                    }
                })
            }
            None => {
                ordering.sort_unstable_by_key(|(_, x)| *x);
            }
        }
        ordering
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The path to the file to read
    yaml_path: PathBuf,

    #[arg(short = 'c', long = "color")]
    colored: bool,

    #[arg(short = 'o', long = "output")]
    stem: PathBuf,

    #[arg(long = "only-check")]
    only_check: bool,
}

fn main() {
    let args = Cli::parse();
    let mut g = Game::new_from_yaml(&args.yaml_path, &args.stem).expect("Parsing failed");

    if args.only_check {
        return
    }

    let start = Instant::now();
    g.sim().unwrap();
    println!("\nRan in {:.2}s", start.elapsed().as_secs_f64());
}
