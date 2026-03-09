// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements a ruleset. This way the game can be played with various different rules.
//! The fore function the ruleset offers is [`RuleSet::iter_perms`] which basically performs the entire
//! simulation.

pub mod parse;

mod permutators;
mod utils;

use crate::matching_repr::bitset::Bitset;
use crate::matching_repr::{IdBase, MaskedMatching};
use crate::ruleset::permutators::{
    dup::add_x_dups_inplace, dup::someone_is_dup_inplace, heaps_permute, n_to_n::n_to_n_inplace,
    trip::add_trip_inplace, trip::someone_is_trip_inplace,
};

use anyhow::{ensure, Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use crate::iterstate::IterStateTrait;
use crate::Lut;

/// data associated with the generic specification of a dupX ruleset
pub type RuleSetDupX = (usize, Vec<String>);

/// An enum defining all the different rulesets which can be applied to the game.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum RuleSet {
    /// A ruleset where X duplicates exist. One of the two individuals forming the dup might be
    /// known (`Some(name)`) or not (`None`).
    /// The dups have to exist on the set_b side.
    XTimesDup(RuleSetDupX),
    /// A ruleset where exactly one triple exists. None of the individuals of the triple is known.
    /// The triple has to exist on the set_b side.
    SomeoneIsTrip,
    /// A ruleset where exactly one triple exists. One of three individuals of the triple is known
    /// The triple has to exist on the set_b side.
    FixedTrip(String),
    /// A ruleset where essentially N:N players play. But there are not really fixed sets a and b.
    /// Instead everyone can match everyone, but it is still a strict 1:1 matching
    NToN,
    /// A ruleset where N:N players play so each individual from set_a matches exactly one
    /// individual from set_b
    #[default]
    Eq,
}

impl RuleSet {
    /// iterate over all permutations derived from the ruleset and perform the simulation with the
    /// help of iterstate `is`
    ///
    /// optionally a `cache` might be used as source for the permutations
    pub fn iter_perms<T: IterStateTrait>(
        &self,
        lut_a: &Lut,
        lut_b: &Lut,
        is: &mut T,
        cache: &Option<PathBuf>,
    ) -> Result<()> {
        is.start();

        // If a cache of serialized MaskedMatching objects exists, prefer streaming that
        // (we deserialize MaskedMatching directly and pass a reference to is.step).
        if let Some(c) = cache {
            let file = File::open(c)
                .with_context(|| format!("Cache path ({:?}) is not a readable file", c))?;
            let reader = BufReader::new(file);
            for (i, line) in reader.lines().enumerate() {
                let p = serde_json::from_str::<MaskedMatching>(&line?)?;
                is.step(i, &p)?;
            }
            is.finish();
            return Ok(());
        }

        // Create one reusable MaskedMatching with the maximal number of slots we will ever emit.
        // Reserve once to avoid reallocation during set_masks_from_slice calls.
        let max_slots = lut_a.len();
        let mut mm = MaskedMatching::with_slots(max_slots);

        // single global index incremented for each emitted permutation
        let mut global_idx: usize = 0;

        match self {
            RuleSet::Eq => {
                // buffer: one singleton Bitset per lut_a index
                let mut buf = (0..lut_a.len() as u8)
                    .map(|i| Bitset::from_idxs(&[i]))
                    .collect::<Vec<_>>();

                // Heaps' permutation over `buf` (in-place), emit each permutation by copying
                // the current `&mut [Bitset]` into the reusable MaskedMatching and calling is.step.
                heaps_permute(&mut buf, |slice| {
                    // emit current permutation
                    let idx = global_idx;
                    global_idx = global_idx
                        .checked_add(1)
                        .context("permutation index overflowed")?;
                    emit_slice_to_state(idx, slice, &mut mm, is)
                })?
            }

            RuleSet::XTimesDup((unknown_cnt, fixed)) => {
                ensure!(
                    lut_b.len() < u8::MAX as usize && lut_a.len() < u8::MAX as usize,
                    "lut too long"
                );
                // build fixed numbers as u8 indices
                let fixed_nums = Bitset::from_idxs(
                    &fixed.iter().map(|d| lut_b[d] as IdBase).collect::<Vec<_>>(),
                );

                // build base vector `x` = all lut_b indices excluding the fixed numbers
                // Len(x) == a + unknown_cnt
                let mut x = (0..lut_b.len() as u8)
                    .filter(|i| !fixed_nums.contains_idx(*i))
                    .map(|i| Bitset::from_idxs(&[i]))
                    .collect::<Vec<_>>();

                let fixed_nums = fixed_nums.iter().collect::<Vec<_>>();

                // outer permutation over x in-place
                heaps_permute(&mut x, |slice| {
                    // slice: &mut [Bitset] of length a + unknown_cnt

                    // distribute the last `unknown_cnt` elements into the first `a` slots
                    someone_is_dup_inplace(slice, *unknown_cnt, |slice| {
                        // slice: &mut [Bitset] of length a

                        // Apply fixed duplicates chain (all `fixed_nums`) in-place.
                        add_x_dups_inplace(slice, &fixed_nums, |slice| {
                            // slice: &mut [Bitset] of length a

                            // emit current permutation
                            let idx = global_idx;
                            global_idx = global_idx
                                .checked_add(1)
                                .context("permutation index overflowed")?;
                            emit_slice_to_state(idx, slice, &mut mm, is)
                        })
                    })
                })?;
            }

            RuleSet::SomeoneIsTrip => {
                let mut base = (0..lut_b.len() as IdBase)
                    .map(|i| Bitset::from_idxs(&[i]))
                    .collect::<Vec<_>>();

                heaps_permute(&mut base, |slice| {
                    someone_is_trip_inplace(slice, |slice| {
                        // emit current permutation
                        let idx = global_idx;
                        global_idx = global_idx
                            .checked_add(1)
                            .context("permutation index overflowed")?;
                        emit_slice_to_state(idx, slice, &mut mm, is)
                    })
                })?;
            }

            RuleSet::FixedTrip(s) => {
                ensure!(
                    lut_b.len() < u8::MAX as usize && lut_a.len() < u8::MAX as usize,
                    "lut too long"
                );
                let fixed_val = *lut_b
                    .get(s)
                    .with_context(|| format!("Invalid index {}", s))?
                    as u8;

                // base buffer: all values except the fixed one
                let mut base = (0..lut_b.len() as IdBase)
                    .filter(|i| *i != fixed_val)
                    .map(|i| Bitset::from_idxs(&[i]))
                    .collect::<Vec<_>>();

                // For every permutation: call add_trip_inplace to insert fixed_val and emit
                heaps_permute(&mut base, |slice| {
                    add_trip_inplace(slice, fixed_val, |slice| {
                        // emit current permutation
                        let idx = global_idx;
                        global_idx = global_idx
                            .checked_add(1)
                            .context("permutation index overflowed")?;
                        emit_slice_to_state(idx, slice, &mut mm, is)
                    })
                })?;
            }

            RuleSet::NToN => {
                n_to_n_inplace(lut_a.len(), |slice| -> anyhow::Result<()> {
                    let idx = global_idx;
                    global_idx = global_idx
                        .checked_add(1)
                        .context("permutation index overflowed")?;
                    emit_slice_to_state(idx, slice, &mut mm, is)
                })?;
            }
        }

        is.finish();
        Ok(())
    }

    /// get the amount of permutations which is to be expected with this ruleset
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

/// Copy `slice` into the provided [`crate::matching_repr::MaskedMatching`] and forward it to the iterator-state.
///
/// When re-using the same MaskedMatching over and over again this avoids having to allocate a
/// MaskedMatching over and over again.
///
/// # Parameters
/// - `idx`: index (position) of this emitted matching within the global enumeration.
/// - `slice`: slice of [`crate::matching_repr::bitset::Bitset`] masks for each slot to be placed into `mm`.
/// - `mm`: a preallocated [`crate::matching_repr::MaskedMatching`] that will be *overwritten* with `slice`.
/// - `is`: mutable reference to an [`crate::iterstate::IterStateTrait`] which will receive the [`crate::matching_repr::MaskedMatching`].
///
/// # Preconditions / Performance
/// - `mm` MUST be preallocated with capacity >= `slice.len()`. Use [`crate::matching_repr::MaskedMatching::with_slots`].
/// - [`crate::matching_repr::MaskedMatching::set_masks_from_slice`] is expected to perform a single `copy_from_slice` style operation
///   (cheap, u64-sized copies) and must not re-allocate in the common case.
///
/// # Notes:
/// - This function is on the hot path; keep it `#[inline]`, allocation-free and minimal.
/// - Do **not** add logging, allocation, or extra cloning here - those would slow hot loops.
///
/// # Returns
/// - Returns the `Result` from `is.step(...)`.
#[inline]
fn emit_slice_to_state<T: IterStateTrait>(
    idx: usize,
    slice: &[Bitset],
    mm: &mut MaskedMatching,
    is: &mut T,
) -> Result<()> {
    mm.set_masks_from_slice(slice); // small cheap memcpy
    is.step(idx, mm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterstate::IterStateTrait;
    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    struct Collector {
        outputs: Vec<MaskedMatching>,
    }

    impl Collector {
        fn new() -> Self {
            Self {
                outputs: Vec::new(),
            }
        }
    }

    impl IterStateTrait for Collector {
        fn start(&mut self) {}
        fn finish(&mut self) {}
        fn step(&mut self, _i: usize, p: &MaskedMatching) -> Result<()> {
            self.outputs.push(p.clone());
            Ok(())
        }
    }

    fn make_lut(values: &[&str]) -> Lut {
        let vec = values
            .iter()
            .enumerate()
            .map(|(i, s)| ((*s).to_string(), i))
            .collect::<Vec<_>>();
        Lut::from_iter(vec)
    }

    #[test]
    fn iter_perms_eq_simple() {
        let lut_a = make_lut(&["a", "b", "c"]);
        let lut_b = lut_a.clone();

        let mut col = Collector::new();
        let rs = RuleSet::Eq;
        rs.iter_perms(&lut_a, &lut_b, &mut col, &None).unwrap();

        assert_eq!(
            col.outputs.len(),
            rs.get_perms_amount(lut_a.len(), lut_b.len(), &None)
                .unwrap()
        );
    }

    #[test]
    fn iter_perms_xtimesdup_simple() {
        let lut_a = make_lut(&["a", "b"]);
        let lut_b = make_lut(&["A", "B", "C", "D"]);

        let cfg = (1, vec!["B".to_string()]);
        let mut col = Collector::new();
        let rs = RuleSet::XTimesDup(cfg);
        rs.iter_perms(&lut_a, &lut_b, &mut col, &None).unwrap();

        assert_eq!(
            col.outputs.len(),
            rs.get_perms_amount(lut_a.len(), lut_b.len(), &None)
                .unwrap()
        );
    }

    #[test]
    fn iter_perms_someonetrip_simple() {
        let lut_a = make_lut(&["a", "b", "c"]);
        let lut_b = make_lut(&["A", "B", "C", "D", "E"]);

        let mut col = Collector::new();
        let rs = RuleSet::SomeoneIsTrip;
        rs.iter_perms(&lut_a, &lut_b, &mut col, &None).unwrap();

        assert_eq!(
            col.outputs.len(),
            rs.get_perms_amount(lut_a.len(), lut_b.len(), &None)
                .unwrap()
        );
    }

    #[test]
    fn iter_perms_fixedtrip_simple() {
        let lut_a = make_lut(&["a", "b", "c"]);
        let lut_b = make_lut(&["A", "B", "C", "D", "E"]);

        let mut col = Collector::new();
        let rs = RuleSet::FixedTrip("B".to_string());
        rs.iter_perms(&lut_a, &lut_b, &mut col, &None).unwrap();

        assert_eq!(
            col.outputs.len(),
            rs.get_perms_amount(lut_a.len(), lut_b.len(), &None)
                .unwrap()
        );
    }

    #[test]
    fn iter_perms_ntoon_simple() {
        let lut_a = make_lut(&["a", "b", "c", "d"]);
        let lut_b = lut_a.clone();

        let mut col = Collector::new();
        let rs = RuleSet::NToN;
        rs.iter_perms(&lut_a, &lut_b, &mut col, &None).unwrap();

        assert_eq!(
            col.outputs.len(),
            rs.get_perms_amount(lut_a.len(), lut_b.len(), &None)
                .unwrap()
        );
    }

    #[test]
    fn iter_perms_cache_simple() {
        let tmp = NamedTempFile::new().unwrap();
        let mm = MaskedMatching::from_matching_ref(&[vec![1, 2], vec![3]]);
        writeln!(tmp.as_file(), "{}", serde_json::to_string(&mm).unwrap()).unwrap();

        let path = PathBuf::from(tmp.path());
        let lut_a = make_lut(&["a"]);
        let lut_b = make_lut(&["b"]);
        let mut col = Collector::new();
        let rs = RuleSet::Eq;
        rs.iter_perms(&lut_a, &lut_b, &mut col, &Some(path.clone()))
            .unwrap();

        assert_eq!(
            col.outputs.len(),
            rs.get_perms_amount(lut_a.len(), lut_b.len(), &Some(path))
                .unwrap()
        );
        assert_eq!(col.outputs[0], mm);
    }

    #[test]
    fn get_perms_amount_eq_simple() {
        let amt = RuleSet::Eq.get_perms_amount(3, 3, &None).unwrap();
        assert_eq!(amt, 6);
    }

    #[test]
    fn get_perms_amount_xtimesdup_simple() {
        let cfg = (1usize, vec!["a".to_string()]);
        let amt = RuleSet::XTimesDup(cfg)
            .get_perms_amount(2, 4, &None)
            .unwrap();
        assert_eq!(amt, 6);
    }

    #[test]
    fn get_perms_amount_someonetrip_simple() {
        let amt = RuleSet::SomeoneIsTrip
            .get_perms_amount(4, 7, &None)
            .unwrap();
        assert_eq!(amt, 4 * (7 * 6 * 5 * 4 * 3 * 2) / 6);
    }

    #[test]
    fn get_perms_amount_fixedtrip_simple() {
        let amt = RuleSet::FixedTrip("a".to_string())
            .get_perms_amount(4, 7, &None)
            .unwrap();
        assert_eq!(amt, 4 * (6 * 5 * 4 * 3 * 2) / 2);
    }

    #[test]
    fn get_perms_amount_ntoon_simple() {
        let amt = RuleSet::NToN.get_perms_amount(4, 4, &None).unwrap();
        assert_eq!(amt, 3);
    }

    #[test]
    fn get_perms_amount_cache_simple() {
        let tmp = NamedTempFile::new().unwrap();
        writeln!(tmp.as_file(), "{{}}").unwrap();

        let path = PathBuf::from(tmp.path());
        let amt = RuleSet::Eq.get_perms_amount(0, 0, &Some(path)).unwrap();
        assert_eq!(amt, 1);
    }
}
