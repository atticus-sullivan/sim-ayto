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

use std::collections::HashMap;
use crate::ruleset::RuleSet;
use crate::Matching;
use anyhow::{Context, Result};


pub trait RuleSetDataClone {
    fn clone_box(&self) -> Box<dyn RuleSetData>;
}
impl<T> RuleSetDataClone for T
where
    T: 'static + RuleSetData + Clone,
{
    fn clone_box(&self) -> Box<dyn RuleSetData> {
        Box::new(self.clone())
    }
}

pub trait RuleSetData: std::fmt::Debug + RuleSetDataClone {
    fn push(&mut self, m: &Matching) -> Result<()>;
    fn print(
        &self,
        full: bool,
        ruleset: &RuleSet,
        map_a: &Vec<String>,
        map_b: &Vec<String>,
        total: u128,
    );
}

impl Clone for Box<dyn RuleSetData> {
    fn clone(&self) -> Box<dyn RuleSetData> {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct DupData {
    cnt: HashMap<(usize, Vec<u8>), usize>,
}

impl std::default::Default for DupData {
    fn default() -> Self {
        Self {
            cnt: Default::default(),
        }
    }
}

impl RuleSetData for DupData {
    fn push(&mut self, m: &Matching) -> Result<()> {
        let k = m
            .iter()
            .enumerate()
            .find(|(_, j)| j.len() > 1)
            .map(|(i, j)| (i, j.clone()))
            .with_context(|| format!(""))?;
        let e = self.cnt.entry(k).or_default();
        *e = *e + 1;
        Ok(())
    }

    fn print(
        &self,
        full: bool,
        ruleset: &RuleSet,
        map_a: &Vec<String>,
        map_b: &Vec<String>,
        total: u128,
    ) {
        let word = match ruleset {
            RuleSet::XTimesDup(_, _) => "Dup",
            RuleSet::SomeoneIsTrip | RuleSet::FixedTrip(_) => "Trip",

            RuleSet::NToN => todo!(),
            RuleSet::Eq => todo!(),
        };

        let mut d = self.cnt.clone().into_iter().collect::<Vec<_>>();
        d.sort_by(|(_, a), (_, b)| b.cmp(a));
        let mut first = true;
        let iter: Box<dyn Iterator<Item = _>> = if full {
            print!("Pr[{word}]: ");
            Box::new(d.into_iter())
        } else {
            print!("top4 Pr[{word}]: ");
            Box::new(d.into_iter().take(4))
        };
        for ((a, bs), cnt) in iter {
            print!(
                "{}{}{:.1}%/{}: {} -> {:?}",
                if full { "\n  " } else { "" },
                if !first { " | " } else { "" },
                (cnt as f64 / total as f64) * 100.0,
                cnt,
                map_a[a],
                bs.iter()
                    .map(|b| map_b[*b as usize].clone())
                    .collect::<Vec<_>>()
            );
            first = false;
        }
        println!();

        let mut d = self
            .cnt
            .iter()
            .fold::<HashMap<Vec<u8>, usize>, _>(HashMap::new(), |mut acc, ((_, js), c)| {
                let x = acc.entry(js.clone()).or_default();
                *x += *c;
                acc
            })
            .into_iter()
            .collect::<Vec<_>>();
        d.sort_by(|(_, a), (_, b)| b.cmp(a));
        let mut first = true;
        let iter: Box<dyn Iterator<Item = _>> = if full {
            print!("Pr[{word}]: ");
            Box::new(d.into_iter())
        } else {
            print!("top5 Pr[{word}]: ");
            Box::new(d.into_iter().take(5))
        };
        for (bs, cnt) in iter {
            print!(
                "{}{}{:.1}%/{}: {:?}",
                if full { "\n  " } else { "" },
                if !first { " | " } else { "" },
                (cnt as f64 / total as f64) * 100.0,
                cnt,
                bs.iter()
                    .map(|b| map_b[*b as usize].clone())
                    .collect::<Vec<_>>()
            );
            first = false;
        }
        println!();

        let mut d = self
            .cnt
            .iter()
            .fold::<HashMap<&u8, usize>, _>(HashMap::new(), |mut acc, ((_, js), c)| {
                for j in js.iter() {
                    let x = acc.entry(j).or_default();
                    *x += *c;
                }
                acc
            })
            .into_iter()
            .collect::<Vec<_>>();
        d.sort_by(|(_, a), (_, b)| b.cmp(a));
        let mut first = true;
        let iter: Box<dyn Iterator<Item = _>> = if full {
            print!("Pr[{word}]: ");
            Box::new(d.into_iter())
        } else {
            print!("top5 Pr[{word}]: ");
            Box::new(d.into_iter().take(5))
        };
        for (b, cnt) in iter {
            print!(
                "{}{}{:.1}%/{}: {}",
                if full { "\n  " } else { "" },
                if !first { " | " } else { "" },
                (cnt as f64 / total as f64) * 100.0,
                cnt,
                map_b[*b as usize]
            );
            first = false;
        }
        println!();

        let mut d = self
            .cnt
            .iter()
            .fold::<HashMap<&usize, usize>, _>(HashMap::new(), |mut acc, ((j, _), c)| {
                let x = acc.entry(j).or_default();
                *x += *c;
                acc
            })
            .into_iter()
            .collect::<Vec<_>>();
        d.sort_by(|(_, a), (_, b)| b.cmp(a));
        let mut first = true;
        let iter: Box<dyn Iterator<Item = _>> = if full {
            print!("Pr[{word}]: ");
            Box::new(d.into_iter())
        } else {
            print!("top5 Pr[{word}]: ");
            Box::new(d.into_iter().take(5))
        };
        for (a, cnt) in iter {
            print!(
                "{}{}{:.1}%/{}: {}",
                if full { "\n  " } else { "" },
                if !first { " | " } else { "" },
                (cnt as f64 / total as f64) * 100.0,
                cnt,
                map_a[*a as usize]
            );
            first = false;
        }
        println!();
    }
}

#[derive(Debug, Clone)]
pub struct DummyData {}

impl std::default::Default for DummyData {
    fn default() -> Self {
        Self {}
    }
}

impl RuleSetData for DummyData {
    fn push(&mut self, _m: &Matching) -> Result<()> {
        Ok(())
    }

    fn print(
        &self,
        _full: bool,
        _ruleset: &RuleSet,
        _map_a: &Vec<String>,
        _map_b: &Vec<String>,
        _total: u128,
    ) {
    }
}

