use indicatif::{ProgressBar, ProgressStyle};

use serde_json::to_writer;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::Result;

use crate::constraint::Constraint;
use crate::Matching;

pub struct IterState {
    pub constraints: Vec<Constraint>,
    pub keep_rem: bool,
    pub each: Vec<Vec<u128>>,
    pub total: u128,
    pub eliminated: u128,
    pub left_poss: Vec<Matching>,
    pub query_matchings: Vec<(Matching, Option<String>)>,
    #[allow(clippy::type_complexity)]
    pub query_pair: (
        HashMap<u8, HashMap<Vec<u8>, u64>>,
        HashMap<u8, HashMap<Vec<u8>, u64>>,
    ),
    cnt_update: usize,
    progress: ProgressBar,
    cache_file: Option<BufWriter<File>>,
}

impl IterState {
    pub fn new(
        keep_rem: bool,
        perm_amount: usize,
        constraints: Vec<Constraint>,
        query_matchings: &[Matching],
        query_pair: &(HashSet<u8>, HashSet<u8>),
        cache_file: &Option<PathBuf>,
        map_lens: (usize, usize),
    ) -> Result<IterState> {
        let file = cache_file
            .clone()
            .map(File::create)
            .map_or(Ok(None), |r| r.map(Some))?
            .map(BufWriter::new);
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

    pub fn start(&mut self) {
        self.progress.inc(0)
    }

    pub fn finish(&mut self) {
        self.progress.finish()
    }

    pub fn step(&mut self, i: usize, p: Matching, output: bool) -> Result<()> {
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

    // count "raw" permutations for statistics and some checks
    fn step_counting_stats(&mut self, p: &Matching) {
        // count how often each pairing occurs without filtering
        // - necessary to be able to work with caching
        // - important to generate the "base-table" from which to calculate how much a constraint
        //   has filtered out / is left after (in percentage)
        for (a, i) in p.iter().enumerate() {
            for b in i.iter() {
                if let Some(x) = self.each.get_mut(a) {
                    if let Some(x_val) = x.get_mut(*b as usize) {
                        *x_val += 1;
                    }
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
    fn step_process(&mut self, p: &Matching) -> Result<bool> {
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
    fn step_collect_query_pair(&mut self, p: &Matching) {
        if !self.query_pair.0.is_empty() || !self.query_pair.1.is_empty() {
            for (a, bs) in p.iter().enumerate() {
                if self.query_pair.0.contains_key(&(a as u8)) {
                    if let Some(val) = self.query_pair.0.get_mut(&(a as u8)) {
                        val.entry(bs.clone())
                            .and_modify(|cnt| *cnt += 1)
                            .or_insert(0);
                    };
                }
                for b in bs.iter() {
                    if let Some(val) = self.query_pair.1.get_mut(b) {
                        val.entry(vec![a as u8])
                            .and_modify(|cnt| *cnt += 1)
                            .or_insert(0);
                    };
                }
            }
        }
    }
}
