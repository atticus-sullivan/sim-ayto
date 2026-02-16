pub mod bitset;
mod conversions;
mod iter;

use serde::{Deserialize, Serialize};

use crate::matching_repr::bitset::Bitset;

pub type Word = u64;
const WORD_BITS: usize = 64;
type IdBase = u8;
// const WORD_BITS_LOG: usize = 6; // log2(64)

/// Public type used by hot code.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MaskedMatching {
    masks: Vec<Bitset>,
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
    /// let m = MaskedMatching::from(&vec![vec![0], vec![]]);
    /// assert!(!m.slot_empty(0));
    /// assert!(m.slot_empty(1));
    /// assert!(m.slot_empty(2)); // out-of-range -> empty
    /// ```
    pub fn slot_empty(&self, slot: usize) -> bool {
        self.masks.get(slot).map(|b| b.is_empty()).unwrap_or(true)
    }

    /// Return the bitset for `slot`.
    pub fn slot_mask(&self, slot: usize) -> Option<&Bitset> {
        self.masks.get(slot)
    }

    /// Prepare a `Vec<Vec<IdBase>>` representation for human-friendly debugging.
    ///
    /// Each slot becomes a `Vec<IdBase>` of the element indices present in that slot.
    pub fn prepare_debug_print(&self) -> Vec<Vec<IdBase>> {
        self.masks.iter().map(|b| b.iter().collect()).collect()
    }

    /// Prepare a name-pair list for debugging: looks up the first element in
    /// each slot (if any) in `map_b` and pairs it with `map_a[a]`.
    ///
    /// # Note
    /// This function assumes `map_a` and `map_b` are large enough to cover the
    /// indices present in the masks. It will panic if an index is out-of-range.
    pub fn prepare_debug_print_names(
        &self,
        map_a: &[String],
        map_b: &[String],
    ) -> Vec<(String, String)> {
        self.masks
            .iter()
            .enumerate()
            .map(|(a, b)| {
                let i = map_a[a as usize].clone();
                let j = map_b[b.iter().next().unwrap() as usize].clone();
                (i, j)
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
    pub fn calculate_lights(&self, sol: &MaskedMatching) -> IdBase {
        let mut l: IdBase = 0;
        for (i, j) in self.masks.iter().enumerate() {
            if j.is_empty() {
                continue;
            }
            if !((sol.masks[i] & *j).is_empty()) {
                l += 1;
            }
        }
        l
    }

    /// Number of slots.
    pub fn len(&self) -> usize {
 self.masks.len()
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
        use std::collections::HashSet;

        use super::*;

        #[test]
        fn test_contains_mask_and_slot_contains_any() {
            let legacy = vec![vec![4u8], vec![2u8]];
            let mm = MaskedMatching::from(&legacy);
            let m = Bitset::from_idxs(&[4u8]);
            assert!(mm.contains_mask(m));
            let mut set = HashSet::new();
            set.insert(3u8);
            set.insert(4u8);
            assert!(mm.slot_mask(0).unwrap().contains_any_idx(&set));
        }

        #[test]
        fn test_prepare_debug_print() {
            let legacy = vec![vec![3u8, 5u8], vec![0u8]];
            let mm = MaskedMatching::from(&legacy);
            let dbg = mm.prepare_debug_print();
            assert_eq!(dbg, legacy);
        }

        #[test]
        fn test_count_matches_singles_and_calculate_lights() {
            let mm = MaskedMatching::from(&vec![vec![0u8], vec![1u8], vec![2u8]]);
            let singles: Vec<IdBase> = vec![0u8, 3u8, 2u8];
            // count_matches_singles equivalent:
            let match_count = mm
                .iter()
                .enumerate()
                .map(|(i, slot)| {
                    let first = singles.get(i).copied().unwrap_or(0);
                    if slot.contains(first) {
                        1
                    } else {
                        0
                    }
                })
                .sum::<usize>() as IdBase;
            assert_eq!(match_count, 2);

            // calculate_lights: compare mm with solution mm2
            let mm2 = MaskedMatching::from(&vec![vec![0u8], vec![1u8], vec![63u8]]);
            assert_eq!(mm.calculate_lights(&mm2), 2u8);
        }

        // Keep this broad: ensure nothing panics for typical scenarios
        #[test]
        fn smoke_test_various_calls() {
            let mm = MaskedMatching::from(&vec![vec![0u8], vec![1u8, 2u8]]);
            let _ = mm.len();
            let _ = mm.computed_universe();
            let _ = mm.prepare_debug_print();
        }
    }
