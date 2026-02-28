//! This module implements an object which statefully manages the whole simulation logic while
//! gathering some statistics along the way.
//! It is also responsible for features like showing a progressbar if this is desired.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::Result;
use indicatif::ProgressStyle;
use serde_json::to_writer;

use crate::constraint::{ConstraintGetters, ConstraintSim};
use crate::matching_repr::IdBase;
use crate::matching_repr::{bitset::Bitset, MaskedMatching};
use crate::progressbar::ProgressBarTrait;

pub(super) type QueryPairData = (
    HashMap<IdBase, HashMap<Bitset, u64>>,
    HashMap<IdBase, HashMap<IdBase, u64>>,
);

/// Trait describing a consumer of emitted matchings during iteration.
///
/// Implementers receive lifecycle calls (`start`, `finish`) and `step` calls for
/// each emitted partial/complete matching. Implement `step` to process or collect
/// results - keep implementations allocation-aware if used in hot paths.
pub trait IterStateTrait {
    /// Called at the start of iteration.
    fn start(&mut self);
    /// Called at the end of iteration.
    fn finish(&mut self);

    /// Called for each emitted matching.
    ///
    /// - `i`: the global sequential index of the emitted matching.
    /// - `p`: the `MaskedMatching` describing the matching.
    /// - `output`: whether this should be treated as an output (verbose/reporting).
    fn step(&mut self, i: usize, p: &MaskedMatching) -> Result<()>;
}

#[derive(Debug)]
pub struct IterState<T: ProgressBarTrait, S: ConstraintSim + ConstraintGetters> {
    pub constraints: Vec<S>,
    pub keep_rem: bool,
    pub each: Vec<Vec<u128>>,
    pub total: u128,
    pub survivors: u128,
    pub left_poss: Vec<MaskedMatching>,
    // allows to query when a Matching was eliminated (by which "comment")
    pub query_matchings: Vec<(MaskedMatching, Option<String>)>,
    #[allow(clippy::type_complexity)]
    pub query_pair: QueryPairData,

    // progressbar related
    cnt_update: usize,
    progress: T,

    cache_file: Option<BufWriter<File>>,
}

/// does not take the constraints into consideration
impl<T: ProgressBarTrait, S: ConstraintSim + ConstraintGetters> PartialEq for IterState<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.keep_rem == other.keep_rem
            && self.each == other.each
            && self.total == other.total
            && self.survivors == other.survivors
            && self.left_poss == other.left_poss
            && self.query_matchings == other.query_matchings
            && self.query_pair == other.query_pair
            && self.cnt_update == other.cnt_update
    }
}

impl<T: ProgressBarTrait, S: ConstraintSim + ConstraintGetters> Default for IterState<T, S> {
    fn default() -> Self {
        Self {
            constraints: Default::default(),
            keep_rem: Default::default(),
            each: Default::default(),
            total: Default::default(),
            survivors: Default::default(),
            left_poss: Default::default(),
            query_matchings: Default::default(),
            query_pair: Default::default(),
            cnt_update: Default::default(),
            progress: T::new(100),
            cache_file: Default::default(),
        }
    }
}

impl<T: ProgressBarTrait, S: ConstraintSim + ConstraintGetters> IterStateTrait for IterState<T, S> {
    /// Start the iteration progress indicator.
    ///
    /// Called at the beginning of an iteration run to initialize progress state.
    fn start(&mut self) {
        self.progress.inc(0)
    }

    /// Finish the iteration progress indicator.
    ///
    /// Called after iteration completes to finalize progress reporting.
    fn finish(&mut self) {
        self.progress.finish()
    }

    /// Process a single permutation step.
    ///
    /// Updates internal statistics and progress for permutation `p` at index `i`.
    /// If `output` is true the progress bar may be advanced.
    fn step(&mut self, i: usize, p: &MaskedMatching) -> Result<()> {
        if i.is_multiple_of(self.cnt_update) {
            self.progress.inc(2);
        }
        self.step_counting_all(p);
        let left = self.step_process(p)?;

        // permutation still works?
        if left {
            self.step_collect_query_pair(p);

            // write permutation to cache file
            if let Some(fs) = &mut self.cache_file {
                to_writer(&mut *fs, p)?;
                writeln!(fs)?;
            }

            self.survivors += 1;

            // store the permutation as still possible solution
            if self.keep_rem {
                self.left_poss.push(p.clone());
            }
        }
        Ok(())
    }
}

impl<T: ProgressBarTrait, S: ConstraintSim + ConstraintGetters> IterState<T, S> {
    /// Create a new `IterState`.
    ///
    /// - `keep_rem`: whether to keep remaining permutations in memory for reporting.
    /// - `perm_amount`: total number of permutations expected (for showing progress).
    /// - `constraints`: list of constraints to apply during iteration.
    /// - `query_matchings`: optional matchings to query/track during iteration.
    /// - `query_pair`: optional pair queries mapping left/right indices to counts.
    pub fn new(
        keep_rem: bool,
        perm_amount: usize,
        constraints: Vec<S>,
        query_matchings: &[MaskedMatching],
        query_pair: &(HashSet<IdBase>, HashSet<IdBase>),
        cache_file: &Option<PathBuf>,
        map_lens: (usize, usize),
    ) -> Result<IterState<T, S>> {
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
            survivors: 0,
            left_poss: vec![],
            progress: T::new(100),
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

    /// Update per-pair counts for statistics from a raw `MaskedMatching`.
    fn step_counting_all(&mut self, p: &MaskedMatching) {
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

    /// Run all constraints for a given permutation.
    ///
    /// Returns `Ok(true)` if the permutation survives all constraints, or `Ok(false)`
    /// if eliminated by any constraint.
    fn step_process(&mut self, p: &MaskedMatching) -> Result<bool> {
        for c in &mut self.constraints {
            if !c.process(p)? {
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

    /// Update query-pair statistics for permutation `p`.
    ///
    /// If a `query_pair` is set, this method increments counters that track how often particular
    /// left/right indices co-occur with specific values.
    fn step_collect_query_pair(&mut self, p: &MaskedMatching) {
        if !self.query_pair.0.is_empty() || !self.query_pair.1.is_empty() {
            for (a, bs) in p.iter().enumerate() {
                if self.query_pair.0.contains_key(&(a as IdBase)) {
                    if let Some(val) = self.query_pair.0.get_mut(&(a as IdBase)) {
                        val.entry(bs).and_modify(|cnt| *cnt += 1).or_insert(1);
                    };
                }
                for b in bs.iter() {
                    if let Some(val) = self.query_pair.1.get_mut(&b) {
                        val.entry(a as IdBase).and_modify(|cnt| *cnt += 1).or_insert(1);
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
    use crate::progressbar::MockProgressBar;

    use pretty_assertions::assert_eq;
    use rust_decimal::Decimal;

    fn sample_matching() -> MaskedMatching {
        // slot0 -> {1,2}, slot1 -> {0}
        MaskedMatching::from_matching_ref(&[vec![1u8, 2u8], vec![0u8]])
    }

    #[derive(Default, Debug, Clone, PartialEq)]
    struct MockConstraint {
        process_cnt: usize,
        fits: bool,
        comment: String,
        type_str: String,
        num: Decimal,
    }

    impl ConstraintSim for MockConstraint {
        fn process(&mut self, _m: &MaskedMatching) -> Result<bool> {
            self.process_cnt += 1;
            Ok(self.fits)
        }
    }
    impl ConstraintGetters for MockConstraint {
        fn comment(&self) -> &str {
            &self.comment
        }

        fn type_str(&self) -> String {
            self.type_str.clone()
        }

        fn num(&self) -> rust_decimal::Decimal {
            self.num
        }
    }

    #[test]
    fn step_counting_all_updates_matrix_and_total() {
        // Build a minimal IterState - only the `each` matrix matters.
        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            each: vec![vec![0; 3]; 2],
            ..Default::default()
        };

        state.step_counting_all(&sample_matching());

        assert_eq!(state.each, vec![vec![0, 1, 1], vec![1, 0, 0],]);
        assert_eq!(state.total, 1);
    }

    #[test]
    fn step_collect_query_pair_populates_maps() {
        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            query_pair: (
                HashMap::from_iter([(0, HashMap::from_iter([]))]),
                HashMap::from_iter([(0, HashMap::from_iter([]))]),
            ),
            ..Default::default()
        };

        state.step_collect_query_pair(&sample_matching());
        state.step_collect_query_pair(&sample_matching());
        state.step_collect_query_pair(&sample_matching());

        assert_eq!(
            state.query_pair.0,
            HashMap::from_iter([(0, HashMap::from_iter([(Bitset::from_idxs(&[1, 2]), 3)])),])
        );
        assert_eq!(
            state.query_pair.1,
            HashMap::from_iter([(0, HashMap::from_iter([(1, 3)])),])
        );
    }

    #[test]
    fn step_process_stores_elimination_comment_for_queried_matching() -> Result<()> {
        // Prepare a query_matchings vector that contains the sample matching.
        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            constraints: vec![MockConstraint {
                fits: false,
                type_str: "TYPE".to_string(),
                comment: "comment".to_string(),
                ..Default::default()
            }],
            query_matchings: vec![(sample_matching(), None)],
            ..Default::default()
        };

        assert!(!state.step_process(&sample_matching())?);

        assert_eq!(
            state.query_matchings,
            vec![(sample_matching(), Some("TYPE comment".to_string())),]
        );

        Ok(())
    }

    #[test]
    fn step_updates_survivors_and_optionally_keeps_remaining() -> Result<()> {
        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            keep_rem: false,
            survivors: 0,
            left_poss: Vec::new(),
            constraints: vec![], // ensures permutation survives
            ..Default::default()
        };

        state.step(0, &sample_matching())?;
        state.step(1, &sample_matching())?;

        assert_eq!(state.survivors, 2);
        assert_eq!(state.left_poss, vec![]);

        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            keep_rem: true,
            survivors: 0,
            left_poss: Vec::new(),
            constraints: vec![], // ensures permutation survives
            ..Default::default()
        };

        state.step(0, &sample_matching())?;
        state.step(1, &sample_matching())?;

        assert_eq!(state.survivors, 2);
        assert_eq!(state.left_poss, vec![sample_matching(), sample_matching(),]);

        Ok(())
    }

    #[test]
    fn step_process_all_constraints_pass() -> Result<()> {
        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            constraints: vec![],
            ..Default::default()
        };
        assert!(state.step_process(&sample_matching())?);

        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            constraints: vec![MockConstraint {
                process_cnt: 0,
                fits: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(state.step_process(&sample_matching())?);
        assert!(state.step_process(&sample_matching())?);

        assert_eq!(
            state.constraints,
            vec![MockConstraint {
                process_cnt: 2,
                fits: true,
                ..Default::default()
            },]
        );
        Ok(())
    }

    #[test]
    fn step_process_multiple() -> Result<()> {
        let mut state: IterState<MockProgressBar, MockConstraint> = IterState {
            constraints: vec![
                MockConstraint {
                    process_cnt: 0,
                    fits: false,
                    ..Default::default()
                },
                MockConstraint {
                    process_cnt: 0,
                    fits: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(!state.step_process(&sample_matching())?);
        assert!(!state.step_process(&sample_matching())?);

        assert_eq!(
            state.constraints,
            vec![
                MockConstraint {
                    process_cnt: 2,
                    fits: false,
                    ..Default::default()
                },
                MockConstraint {
                    process_cnt: 0,
                    fits: true,
                    ..Default::default()
                },
            ]
        );
        Ok(())
    }
}
