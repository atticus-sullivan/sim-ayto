use crate::{matching_repr::MaskedMatching, ruleset::RuleSet, matching_repr::bitset::Bitset};
use crate::ruleset_data::RuleSetData;
use crate::Lut;
use anyhow::{Context, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct DupData {
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

        let e = self.cnt.entry(k).or_default();
        *e += 1;

        Ok(())
    }

    // TODO: split up + output
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

            RuleSet::NToN => todo!(),
            RuleSet::Eq => todo!(),
        };

        let mut d = self.cnt.clone().into_iter().collect::<Vec<_>>();
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
                (cnt as f64 / total as f64) * 100.0,
                cnt,
                map_a[a],
                bs.iter()
                    .map(|b| map_b[b as usize].clone())
                    .collect::<Vec<_>>()
            );
            first = false;
        }
        println!();

        let mut d = self
            .cnt
            .iter()
            .fold::<HashMap<Bitset, usize>, _>(HashMap::new(), |mut acc, ((_, js), c)| {
                let x = acc.entry(*js).or_default();
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
                    .map(|b| map_b[b as usize].clone())
                    .collect::<Vec<_>>()
            );
            first = false;
        }
        println!();

        let mut d = self
            .cnt
            .iter()
            .fold::<HashMap<u8, usize>, _>(HashMap::new(), |mut acc, ((_, js), c)| {
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
                map_b[b as usize]
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
