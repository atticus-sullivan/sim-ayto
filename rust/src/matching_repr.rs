// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements a representation for storing a full matching.

pub mod bitset;
mod conversions;
mod iter;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::matching_repr::bitset::Bitset;

/// The type used to store the bitset - a variable so the amount of available bits can easily be
/// increased
pub type Word = u64;
/// The type used to store elements in the index representation
pub type IdBase = u8;
/// the amount of bits available in the currently used `Word`
const WORD_BITS: usize = 64;
// const WORD_BITS_LOG: usize = 6; // log2(64)

/// typical maximum length of matchings relevant for using smallvec
/// don't use 10 to leave space in case the permutation generation code needs space for more (e.g.
/// the dups/trips in the initial permutation)
pub const MATCH_MAX_LEN: usize = 15;

/// This is a way of representing a full matching. For each element it stores a `Bitset` so multple
/// items from *set_b* can match to the element (from *set_a*).
///
/// This is much cheaper than storing Vec<Vec<IdBase>>. This way it becomes a simple Vec<Bitset>.
/// Usually a matching will not have more than `MATCH_MAX_LEN` (~10) slots. Thus, internally we use
/// a `SmallVec` instead of a `Vec`. When storing a list of matchings (Vec<MaskedMatching>), this
/// saves us from pointer chasing and leads to this list of matching being stored consecutive in
/// memory (not lots of tiny allocations).
/// Still `SmallVec` can also store larger vectors. In such a case it resorts back to doing a heap
/// allocation. It also supports storing vectors/arrays with do not take up the full length.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MaskedMatching {
    /// mapping idx_a to a set of idxs_b where the idxs_b are stored efficiently in a
    /// bitmask/bitset
    masks: SmallVec<[Bitset; MATCH_MAX_LEN]>,
}

impl MaskedMatching {
    /// Returns whether the slot at `slot` contains no values.
    ///
    /// The function never panics for out-of-range `slot`: if `slot` is greater
    /// than the number of slots it returns `true` (empty).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ayto::matching_repr::MaskedMatching;
    /// let m = MaskedMatching::from_matching_ref(&vec![vec![0], vec![]]);
    /// assert!(!m.slot_empty(0));
    /// assert!(m.slot_empty(1));
    /// assert!(m.slot_empty(2)); // out-of-range -> empty
    /// ```
    pub fn slot_empty(&self, slot: usize) -> bool {
        self.masks.get(slot).map(|b| b.is_empty()).unwrap_or(true)
    }

    /// Return the bitset for `slot` if it exists.
    /// If the slot does not exist, `None` is returned.
    pub fn slot_mask(&self, slot: usize) -> Option<&Bitset> {
        self.masks.get(slot)
    }

    /// Prepare a `Vec<Vec<IdBase>>` representation for human-friendly printing.
    ///
    /// Each slot becomes a `Vec<IdBase>` of the element indices present in that slot.
    /// The idea is that this can be printed via the Debug trait.
    pub fn prepare_debug_print(&self) -> Vec<Vec<IdBase>> {
        self.masks.iter().map(|b| b.iter().collect()).collect()
    }

    /// Prepare a name-pair list for human-friendly printing, just like `prepare_debug_print`. The
    /// difference is that this function resolves the indices to the strings/names.
    ///
    /// # Note
    /// This function assumes `map_a` and `map_b` are large enough to cover the
    /// indices present in the masks. It will panic if an index is out-of-range.
    pub fn prepare_debug_print_names(
        &self,
        map_a: &[String],
        map_b: &[String],
    ) -> Result<Vec<(String, Vec<String>)>> {
        self.masks
            .iter()
            .enumerate()
            .map(|(a, b)| {
                let i = map_a[a].clone();
                let j = b.iter().map(|b| map_b[b as usize].clone()).collect();
                Ok((i, j))
            })
            .collect()
    }

    /// Returns `true` if any slot contains the exact `mask`.
    pub fn contains_mask(&self, mask: Bitset) -> bool {
        self.masks.iter().any(|b| b.0 == mask.0)
    }

    /// Counts the number of slots that are non-empty and overlap with `sol`.
    ///
    /// This method compares the calling `MaskedMatching` to `sol` and returns
    /// how many slots have a non-zero intersection.
    ///
    /// Note: In the context of the game this is also known as how many lights the calling Matching
    /// gets with `sol` as the solution.
    pub fn calculate_lights(&self, sol: &MaskedMatching) -> IdBase {
        let mut l: IdBase = 0;
        let a: &[Bitset] = &self.masks;
        let b: &[Bitset] = &sol.masks;
        for i in 0..a.len() {
            // try to help the comiler using vector instructions here
            l += a[i].contains_any(b[i]) as IdBase;
        }
        l
    }

    /// Number of slots available (doesn't check if the slots are empty or not).
    pub fn len(&self) -> usize {
        self.masks.len()
    }

    /// No slots?
    pub fn is_empty(&self) -> bool {
        self.masks.is_empty()
    }

    /// Compute the universe (highest set bit + 1) or 0 if empty.
    ///
    /// Useful when converting back to `Vec<Vec<IdBase>>` to know the upper bound.
    pub fn computed_universe(&self) -> usize {
        self.masks
            .iter()
            .map(|b| {
                if b.is_empty() {
                    0usize
                } else {
                    // highest set bit index + 1
                    let w = b.as_word();
                    (WORD_BITS - 1) - (w.leading_zeros() as usize) + 1
                }
            })
            .max()
            .unwrap_or(0)
    }
}

impl<'a> std::ops::BitAnd<&'a MaskedMatching> for MaskedMatching {
    type Output = Self;

    fn bitand(mut self, rhs: &'a MaskedMatching) -> Self::Output {
        for (i, j) in self.masks.iter_mut().zip(&rhs.masks) {
            *i &= *j;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn contains_mask_simple() {
        let mm = MaskedMatching::from_matching_ref(&[vec![4u8], vec![2u8]]);
        let m = Bitset::from_idxs(&[4u8]);
        assert!(mm.contains_mask(m));
        let m = Bitset::from_idxs(&[3u8]);
        assert!(!mm.contains_mask(m));
    }

    #[test]
    fn prepare_debug_print_simple() {
        let m = vec![vec![3u8, 5u8], vec![0u8]];
        let mm = MaskedMatching::from_matching_ref(&m);
        let dbg = mm.prepare_debug_print();
        assert_eq!(dbg, m);
    }

    #[test]
    fn calculate_lights_simple() {
        let mm1 = MaskedMatching::from_matching_ref(&[vec![0u8], vec![1u8], vec![2u8]]);
        let mm2 = MaskedMatching::from_matching_ref(&[vec![0u8], vec![1u8], vec![63u8]]);
        assert_eq!(mm1.calculate_lights(&mm2), 2u8);
    }

    #[test]
    fn slot_empty_simple() {
        let mm = MaskedMatching::from_matching_ref(&[vec![0u8], vec![]]);
        assert!(!mm.slot_empty(0));
        assert!(mm.slot_empty(1));
        assert!(mm.slot_empty(2));
    }

    #[test]
    fn slot_mask_simple() {
        let mm = MaskedMatching::from_matching_ref(&[vec![], vec![1u8]]);
        assert!(mm.slot_mask(0).is_some());
        assert!(mm.slot_mask(1).is_some());
        assert!(mm.slot_mask(2).is_none());
    }

    #[test]
    fn prepare_debug_print_names_non_empty() {
        let mm = MaskedMatching::from_matching_ref(&[vec![2u8], vec![0u8, 1u8]]);
        let map_a = vec!["A".into(), "B".into()];
        let map_b = vec!["a".into(), "b".into(), "c".into()];
        let names = mm.prepare_debug_print_names(&map_a, &map_b).unwrap();
        assert_eq!(
            names,
            vec![
                ("A".into(), vec!["c".into()]),
                ("B".into(), vec!["a".into(), "b".into()])
            ]
        );
    }

    #[test]
    fn calculate_lights_all() {
        let mm1 = MaskedMatching::from_matching_ref(&[vec![0u8], vec![1u8]]);
        let mm2 = MaskedMatching::from_matching_ref(&[vec![0u8], vec![1u8]]);
        assert_eq!(mm1.calculate_lights(&mm2), 2u8);
    }

    #[test]
    fn calculate_lights_none() {
        let mm1 = MaskedMatching::from_matching_ref(&[vec![0u8], vec![1u8]]);
        let mm2 = MaskedMatching::from_matching_ref(&[vec![2u8], vec![3u8]]);
        assert_eq!(mm1.calculate_lights(&mm2), 0u8);
    }

    #[test]
    fn len_simple() {
        let mm = MaskedMatching::from_matching_ref(&[vec![0u8], vec![1u8]]);
        assert_eq!(mm.len(), 2);
        let mm = MaskedMatching::from_matching_ref(&[]);
        assert_eq!(mm.len(), 0);
    }

    #[test]
    fn is_empty_simple() {
        let mm_empty = MaskedMatching::from_matching_ref(&[]);
        let mm_non = MaskedMatching::from_matching_ref(&[vec![0u8]]);
        assert!(mm_empty.is_empty());
        assert!(!mm_non.is_empty());
    }

    #[test]
    fn computed_universe_simple() {
        let mm = MaskedMatching::from_matching_ref(&[]);
        assert_eq!(mm.computed_universe(), 0);
        let mm = MaskedMatching::from_matching_ref(&[vec![0u8, 5u8]]);
        assert_eq!(mm.computed_universe(), 6);
    }

    #[test]
    fn bit_and_simple() {
        let left = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(0b101),
            Bitset::from_word(0b110),
        ]));
        let right = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(0b011),
            Bitset::from_word(0b100),
        ]));
        let expected = MaskedMatching::from_masks(SmallVec::from_slice(&[
            Bitset::from_word(0b001),
            Bitset::from_word(0b100),
        ]));
        let result = left & &right;
        assert_eq!(result, expected);
    }
}
