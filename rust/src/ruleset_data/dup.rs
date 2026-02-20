use crate::ruleset_data::RuleSetData;
use crate::Lut;
use crate::{matching_repr::bitset::Bitset, matching_repr::MaskedMatching, ruleset::RuleSet};
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Collect statistics about "dup" (or "trip") events.
///
/// Internally keeps a map from `(index_in_set_a, bitset_of_b_indices)` -> count.
#[derive(Debug, Clone, Default)]
pub struct DupData {
    cnt: HashMap<(usize, Bitset), usize>,
}

impl DupData {
    /// Aggregate the counts by `Bitset` (set of b indices) and return
    /// a vector of `(Bitset, count)` sorted by count desc.
    fn aggregate_by_bitset(&self) -> Vec<(Bitset, usize)> {
        let mut agg: HashMap<Bitset, usize> = HashMap::new();
        for ((_, js), c) in self.cnt.iter() {
            *agg.entry(*js).or_default() += *c;
        }
        let mut v: Vec<_> = agg.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    /// Aggregate counts by individual b-indices (how often a single b appears in duplicates)
    /// Returns `Vec<(b_index, count)>` sorted by count desc.
    fn aggregate_by_b(&self) -> Vec<(u8, usize)> {
        let mut agg: HashMap<u8, usize> = HashMap::new();
        for ((_, js), c) in self.cnt.iter() {
            for j in js.iter() {
                *agg.entry(j).or_default() += *c;
            }
        }
        let mut v: Vec<_> = agg.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }
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

    // TODO: split
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

        let mut d = self.aggregate_by_bitset();
        let mut first = true;
        if full {
            print!("Pr[{word}]: ");
        } else {
            print!("top5 Pr[{word}]: ");
            d.truncate(5);
        };
        for (bs, cnt) in d {
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

        let mut d = self.aggregate_by_b();
        let mut first = true;
        if full {
            print!("Pr[{word}]: ");
        } else {
            print!("top5 Pr[{word}]: ");
            d.truncate(5);
        };
        for (b, cnt) in d {
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
        if full {
            print!("Pr[{word}]: ");
        } else {
            print!("top5 Pr[{word}]: ");
            d.truncate(5);
        };
        for (a, cnt) in d {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching_repr::MaskedMatching;

    #[test]
    fn dupdata_push_and() {
        let mut d = DupData::default();

        // create a matching where index 0 duplicates over b indices 1 and 2
        let m = MaskedMatching::from_matching_ref(&[vec![1u8, 2u8]]);
        d.push(&m).unwrap();

        // aggregate by bitset: the single bitset should exist with count 1
        let agg = d.aggregate_by_bitset();
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].1, 1);

        // aggregate by b -> both b indices counted once each
        let mut ab = d.aggregate_by_b();
        ab.sort_by_key(|(b, _)| *b);
        assert_eq!(
            ab.iter().map(|(b, _)| *b).collect::<Vec<_>>(),
            vec![1u8, 2u8]
        );
    }
}
