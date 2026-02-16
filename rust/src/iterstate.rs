use indicatif::{ProgressBar, ProgressStyle};

use serde_json::to_writer;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::Result;

use crate::constraint::Constraint;
use crate::matching_repr::{bitset::Bitset, MaskedMatching};

pub struct IterState {
    pub constraints: Vec<Constraint>,
    pub keep_rem: bool,
    pub each: Vec<Vec<u128>>,
    pub total: u128,
    pub eliminated: u128,
    pub left_poss: Vec<MaskedMatching>,
    // allows to query when a Matching was eliminated (by which "comment")
    pub query_matchings: Vec<(MaskedMatching, Option<String>)>,
    #[allow(clippy::type_complexity)]
    pub query_pair: (
        HashMap<u8, HashMap<Bitset, u64>>,
        HashMap<u8, HashMap<u8, u64>>,
    ),
    cnt_update: usize,
    progress: ProgressBar,
    cache_file: Option<BufWriter<File>>,
}

pub trait IterStateTrait {
    fn start(&mut self);
    fn finish(&mut self);

    fn step(&mut self, i: usize, p: MaskedMatching, output: bool) -> Result<()>;
}

impl IterStateTrait for IterState {
    fn start(&mut self) {
        self.progress.inc(0)
    }

    fn finish(&mut self) {
        self.progress.finish()
    }

    fn step(&mut self, i: usize, p: MaskedMatching, output: bool) -> Result<()> {
        if i.is_multiple_of(self.cnt_update) && output {
            self.progress.inc(2);
        }
        self.step_counting_stats(&p);
        let left = self.step_process(&p)?;

        // permutation still works?
        if left {
            self.step_collect_query_pair(&p);

            // write permutation to cache file
            if let Some(fs) = &mut self.cache_file {
                to_writer(&mut *fs, &p)?;
                writeln!(fs)?;
            }

            // store the permutation as still possible solution
            if self.keep_rem {
                self.left_poss.push(p);
            }
        }
        Ok(())
    }
}

impl IterState {
    pub fn new(
        keep_rem: bool,
        perm_amount: usize,
        constraints: Vec<Constraint>,
        query_matchings: &[MaskedMatching],
        query_pair: &(HashSet<u8>, HashSet<u8>),
        cache_file: &Option<PathBuf>,
        map_lens: (usize, usize),
    ) -> Result<IterState> {
        let file = if let Some(path) = cache_file {
            Some(BufWriter::new(File::create(path)?))
        } else {
            None
        };
        let is = IterState {
            constraints,
            keep_rem,
            query_matchings: query_matchings.iter().map(|i| (i.clone(), None)).collect(),
            query_pair: (
                query_pair
                    .0
                    .iter()
                    .map(|i| (*i, Default::default()))
                    .collect(),
                query_pair
                    .1
                    .iter()
                    .map(|i| (*i, Default::default()))
                    .collect(),
            ),
            each: vec![vec![0; map_lens.1]; map_lens.0],
            total: 0,
            eliminated: 0,
            left_poss: vec![],
            progress: ProgressBar::new(100),
            cnt_update: std::cmp::max(perm_amount / 50, 1),
            cache_file: file,
        };
        is.progress.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar}] {pos:>3}/{len:3} (ETA: {eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );
        Ok(is)
    }

    // count "raw" permutations for statistics and some checks
    fn step_counting_stats(&mut self, p: &MaskedMatching) {
        // count how often each pairing occurs without filtering
        // - necessary to be able to work with caching
        // - important to generate the "base-table" from which to calculate how much a constraint
        //   has filtered out / is left after (in percentage)
        for (k, v) in p.iter_pairs() {
            if let Some(x) = self.each.get_mut(k as usize) {
                if let Some(x_val) = x.get_mut(v as usize) {
                    *x_val += 1;
                }
            }
        }
        // aggregate to check the (mathematically) calculated total permutations count
        self.total += 1;
    }

    // loop over constraints feeding them the permutation and check if the permutation still
    // works with that constraint
    //
    // returns: is this permutation still possible with this set of constraints
    fn step_process(&mut self, p: &MaskedMatching) -> Result<bool> {
        for c in &mut self.constraints {
            if !c.process(p)? {
                // permutation was eliminated by this constraint
                self.eliminated += 1;
                // check if this permutation was queried.
                // If so store by which constraint it was eliminated
                for (q, id) in &mut self.query_matchings {
                    if q == p {
                        *id = Some(c.type_str().to_string() + " " + c.comment());
                    }
                }
                return Ok(false);
            }
        }
        Ok(true)
    }

    // allow to query with person A / person B can still be a match (and how often)
    fn step_collect_query_pair(&mut self, p: &MaskedMatching) {
        if !self.query_pair.0.is_empty() || !self.query_pair.1.is_empty() {
            for (a, bs) in p.iter().enumerate() {
                if self.query_pair.0.contains_key(&(a as u8)) {
                    if let Some(val) = self.query_pair.0.get_mut(&(a as u8)) {
                        val.entry(bs).and_modify(|cnt| *cnt += 1).or_insert(1);
                    };
                }
                for b in bs.iter() {
                    if let Some(val) = self.query_pair.1.get_mut(&b) {
                        val.entry(a as u8).and_modify(|cnt| *cnt += 1).or_insert(1);
                    };
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching_repr::MaskedMatching;
    use crate::matching_repr::bitset::Bitset;
    use std::collections::HashSet;

    // A tiny helper to create a MaskedMatching easily.
    fn mk_mm() -> MaskedMatching {
        // slot0: {1,2}, slot1: {0}
        MaskedMatching::from(&vec![vec![1u8, 2u8], vec![0u8]])
    }

    #[test]
    fn iterstate_step_counts_each_and_total_and_query_pair() {
        // Create an IterState with:
        // - no constraints
        // - query_pair tracking slot 0 and value index 0 (for right-hand map)
        let mut query_slots = HashSet::new();
        query_slots.insert(0u8); // track slot 0 keyed by slot index
        let mut query_values = HashSet::new();
        query_values.insert(0u8); // track value 0 keyed by value index

        let mut is = IterState::new(
            false,               // keep_rem
            10,                  // perm_amount (only used to size progress, cnt_update)
            Vec::new(),          // constraints
            &[],                 // query_matchings
            &(query_slots, query_values),
            &None,               // cache_file
            (2usize, 4usize),    // map_lens: 2 slots, universe size 4
        )
        .expect("failed to create IterState");

        // perform a step with our test permutation
        let p = mk_mm();
        // step returns Result<()>
        is.step(0usize, p.clone(), false).expect("step failed");

        // total should have incremented
        assert_eq!(is.total, 1u128);

        // each[slot][value] should reflect our permutation:
        // slot 0 had values 1 and 2 -> each[0][1] == 1 and each[0][2] == 1
        assert_eq!(is.each[0][1], 1u128);
        assert_eq!(is.each[0][2], 1u128);

        // check query_pair maps: because we tracked slot 0 (query_pair.0 contains 0),
        // there should be an entry with key = Bitset corresponding to slot mask {1,2}
        if let Some(slot_map) = is.query_pair.0.get(&0u8) {
            let mask = Bitset::from_idxs(&[1u8, 2u8]);
            // the value for that Bitset should be 1 (first observation)
            assert_eq!(slot_map.get(&mask).copied().unwrap_or(0), 1u64);
        } else {
            panic!("expected query_pair.0 to contain key 0");
        }

        // and because we tracked value 0 on right-side, the second map should record
        // how often value index 0 occurred and at which slot(s) (we had value 0 at slot 1)
        if let Some(val_map) = is.query_pair.1.get(&0u8) {
            // the slot index 1 should be recorded (value 0 was present at slot1)
            assert_eq!(val_map.get(&1u8).copied().unwrap_or(0), 1u64);
        } else {
            panic!("expected query_pair.1 to contain key 0");
        }
    }
}
