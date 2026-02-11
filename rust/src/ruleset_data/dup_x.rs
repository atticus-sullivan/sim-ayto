use crate::ruleset::RuleSet;
use crate::ruleset::RuleSetDupX;
use crate::ruleset_data::RuleSetData;
use crate::Lut;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct DupXData {
    // HashMap.key: (index in set_a maps to [...])
    // HashMap.value: count
    cnt: HashMap<(usize, Vec<u8>), usize>,
    rs: RuleSetDupX,
}

impl DupXData {
    pub fn new(rs: RuleSetDupX) -> Result<Self> {
        Ok(Self {
            cnt: HashMap::default(),
            rs: rs.clone(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    // TODO: split up + output
    fn print_one(
        &self,
        full: bool,
        ruleset: &RuleSet,
        map_a: &[String],
        map_b: &[String],
        total: u128,
        query: Option<u8>,
        query_not: &HashSet<u8>,
    ) -> Result<()> {
        let word = match ruleset {
            RuleSet::XTimesDup(_) => "Dup",
            RuleSet::SomeoneIsTrip | RuleSet::FixedTrip(_) => "Trip",

            RuleSet::NToN => todo!(),
            RuleSet::Eq => todo!(),
        };

        let mut d = match query {
            Some(q) => self
                .cnt
                .iter()
                .filter(|i| i.0 .1.contains(&q) && i.0 .1.iter().all(|j| !query_not.contains(j)))
                .collect::<Vec<_>>(),
            None => self
                .cnt
                .iter()
                .filter(|i| i.0 .1.iter().all(|j| !query_not.contains(j)))
                .collect::<Vec<_>>(),
        };

        let cnt_filtered = d.clone();

        // print all (or the most common) full duplicate matchings
        d.sort_by(|(ass, a), (bss, b)| b.cmp(a).then_with(|| ass.cmp(bss)));
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
                (*cnt as f64 / total as f64) * 100.0,
                cnt,
                map_a[*a],
                bs.iter()
                    .map(|b| map_b[*b as usize].clone())
                    .collect::<Vec<_>>()
            );
            first = false;
        }
        println!();

        // print all (or the most common) people (set_b) together being part of the duplicate matching
        let mut d = cnt_filtered
            .iter()
            .fold::<HashMap<Vec<u8>, usize>, _>(HashMap::new(), |mut acc, ((_, js), c)| {
                let x = acc.entry(js.clone()).or_default();
                *x += *c;
                acc
            })
            .into_iter()
            .collect::<Vec<_>>();
        d.sort_by(|(ass, a), (bss, b)| b.cmp(a).then_with(|| ass.cmp(bss)));
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

        // print all (or the most common) people (set_a) being part of the duplicate matching
        let mut d = cnt_filtered
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
        d.sort_by(|(ass, a), (bss, b)| b.cmp(a).then_with(|| ass.cmp(bss)));
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
        println!("   / 200 %");

        // print all (or the most common) people (set_b) being part of the duplicate matching
        let mut d = cnt_filtered
            .iter()
            .fold::<HashMap<&usize, usize>, _>(HashMap::new(), |mut acc, ((j, _), c)| {
                let x = acc.entry(j).or_default();
                *x += *c;
                acc
            })
            .into_iter()
            .collect::<Vec<_>>();
        d.sort_by(|(ass, a), (bss, b)| b.cmp(a).then_with(|| ass.cmp(bss)));
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
                map_a[*a]
            );
            first = false;
        }
        println!();
        Ok(())
    }
}

impl RuleSetData for DupXData {
    fn push(&mut self, m: &[Vec<u8>]) -> Result<()> {
        let ks = m
            .iter()
            .enumerate()
            .filter(|(_, j)| j.len() > 1)
            .map(|(i, j)| (i, j.clone()));
        for k in ks {
            let e = self.cnt.entry(k).or_default();
            *e += 1;
        }
        Ok(())
    }

    // TODO: output?
    fn print(
        &self,
        full: bool,
        ruleset: &RuleSet,
        map_a: &[String],
        map_b: &[String],
        lut_b: &Lut,
        total: u128,
    ) -> Result<()> {
        for d in self.rs.1.iter() {
            println!("Pr[]s for dup with {d}");
            let not = self
                .rs
                .1
                .iter()
                .filter_map(|i| {
                    if i != d {
                        Some(
                            lut_b
                                .get(i)
                                .map(|x| *x as u8)
                                .with_context(|| format!("{i} not found")),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Result<HashSet<_>>>()?;
            let q = lut_b
                .get(d)
                .map(|d| *d as u8)
                .with_context(|| format!("{d} not found"))?;
            self.print_one(full, ruleset, map_a, map_b, total, Some(q), &not)
                .unwrap();
            println!(".");
        }

        if self.rs.0 > 0 {
            println!("Pr[]s for unknown dup");
            let not = self
                .rs
                .1
                .iter()
                .map(|i| {
                    lut_b
                        .get(i)
                        .map(|x| *x as u8)
                        .with_context(|| format!("{i} not found"))
                })
                .collect::<Result<HashSet<_>>>()?;
            self.print_one(full, ruleset, map_a, map_b, total, None, &not)
                .unwrap();
        }
        Ok(())
    }
}
