/// This module allows to iterate in various different ways over a MaskedMatching.
use crate::matching_repr::{
    bitset::{BitIter, Bitset},
    IdBase, MaskedMatching, Word, WORD_BITS,
};

impl MaskedMatching {
    /// Iterate over slots: returns an iterator of `Bitset` (one bitset per slot).
    pub fn iter(&self) -> SlotsIter<'_> {
        SlotsIter {
            masks: &self.masks,
            idx: 0,
        }
    }

    /// Iterate over (slot,value) pairs: yields `(slot_index, value_index)`.
    pub fn iter_pairs(&self) -> PairsIter<'_> {
        PairsIter {
            masks: &self.masks,
            idx: (0, 0),
        }
    }

    fn build_bit_iters(&self) -> Vec<BitIter> {
        self.masks.iter().map(|b| b.iter()).collect()
    }

    fn prime_iters(iters: &mut [BitIter]) -> Vec<Option<IdBase>> {
        iters.iter_mut().map(|it| it.next()).collect()
    }

    /// Iterate over combinations that pick exactly one value from each slot,
    /// producing `MaskedMatching` objects that have single-bit masks per slot.
    pub fn iter_unwrapped(&self) -> UnwrappedIter<'_> {
        let mut iters = self.build_bit_iters();
        let current = Self::prime_iters(&mut iters);
        let done = current.iter().any(|x| x.is_none()) || current.is_empty();

        UnwrappedIter {
            mm: self,
            iters,
            current,
            done,
        }
    }
}

/// Iterator over pairs (slot, value).
///
/// PairsIter yields `(slot_index, value_index)` for every set bit in each slot.
/// The iteration order is: increasing slot index; within a slot increasing value index.
pub struct PairsIter<'a> {
    masks: &'a [Bitset],
    idx: (usize, usize), // (slot_index, bit-search-start)
}

impl<'a> Iterator for PairsIter<'a> {
    type Item = (IdBase, IdBase);
    /// Iterator `next` implementation returning the next item or `None`.
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.masks.len();
        loop {
            // If we've exhausted slots, terminate.
            if self.idx.0 >= len {
                return None;
            }

            // Read the current slot's word.
            let slot_word: Word = self.masks[self.idx.0].as_word();

            // If our inner search index is past the word width, move to next slot.
            if self.idx.1 >= WORD_BITS {
                self.idx = (self.idx.0 + 1, 0);
                continue;
            }

            // Mask off bits below the current inner index.
            let mask = slot_word & ((!0u64) << (self.idx.1));

            // If no bits remain at/after idx.1, advance to next slot.
            if mask == 0 {
                self.idx = (self.idx.0 + 1, 0);
                continue;
            }

            // lowest set bit index in mask
            let cur_val = mask.trailing_zeros() as IdBase;

            // remove the found lowest bit from mask to compute the next inner index
            let after_mask = mask & (mask - 1);
            let next_inner = if after_mask == 0 {
                // No more bits in this slot after the found one -> mark as past-word
                WORD_BITS
            } else {
                after_mask.trailing_zeros() as usize
            };

            let slot_idx = self.idx.0;
            self.idx = (slot_idx, next_inner);

            return Some((slot_idx as IdBase, cur_val));
        }
    }
}

/// Slots iterator: returns `Bitset` per slot (so iterating a MaskedMatching yields Bitset).
pub struct SlotsIter<'a> {
    masks: &'a [Bitset],
    idx: usize,
}
impl<'a> Iterator for SlotsIter<'a> {
    type Item = Bitset;
    /// Iterator `next` implementation returning the next item or `None`.
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.masks.len() {
            None
        } else {
            let cur = self.idx;
            self.idx += 1;
            Some(self.masks[cur])
        }
    }
}

/// Cartesian-product iterator: pick exactly one value per slot.
/// If any slot is empty the product is empty (iterator yields None immediately).
pub struct UnwrappedIter<'a> {
    mm: &'a MaskedMatching,
    /// Current iterators for each slot
    iters: Vec<BitIter>,
    /// Current selection (indices of bits per slot)
    current: Vec<Option<IdBase>>,
    /// Flag to indicate iteration is done
    done: bool,
}

impl<'a> Iterator for UnwrappedIter<'a> {
    type Item = MaskedMatching;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let result = self.build_current_masked();
        if !self.advance() {
            self.done = true;
        }
        Some(result)
    }
}

impl UnwrappedIter<'_> {
    fn build_current_masked(&self) -> MaskedMatching {
        // Build single-bit mask slots from current selection
        let slots = self
            .current
            .iter()
            .map(|&opt_idx| opt_idx.map_or_else(Bitset::empty, |idx| Bitset::from_idxs(&[idx])))
            .collect::<Vec<Bitset>>();
        MaskedMatching::from_masks(slots)
    }

    fn advance(&mut self) -> bool {
        // Advance to next combination: try to advance from the rightmost slot.
        // When we reset iterators to the right, we must advance the *real*
        // iterator (call `.next()` on it) so `iters` and `current` remain synced.
        for i in (0..self.iters.len()).rev() {
            if let Some(next_idx) = self.iters[i].next() {
                self.current[i] = Some(next_idx);
                // reset all iterators to the right and consume their first element
                for j in i + 1..self.iters.len() {
                    self.iters[j] = BitIter {
                        w: *self.mm.slot_mask(j).unwrap(),
                    };
                    // IMPORTANT: advance the real iterator and store its value
                    self.current[j] = self.iters[j].next();
                }
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;

    #[test]
    fn iter_basic() {
        let mm = MaskedMatching::from_matching_ref(&[vec![1, 2], vec![0]]);
        let masks: Vec<Bitset> = mm.iter().collect();

        assert_eq!(
            masks,
            vec![Bitset::from_idxs(&[1, 2]), Bitset::from_idxs(&[0])]
        )
    }

    #[test]
    fn iter_pairs_order() {
        let mm = MaskedMatching::from_matching_ref(&[vec![1, 2], vec![0]]);
        let pairs: Vec<(IdBase, IdBase)> = mm.iter_pairs().collect();
        assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 0)]);
    }

    #[test]
    fn iter_unwrapped_cartesian_product() {
        let mm = MaskedMatching::from_matching_ref(&[vec![0, 1], vec![2, 3]]);

        let combos: Vec<Vec<Vec<IdBase>>> = mm
            .iter_unwrapped()
            .map(|m| Vec::try_from(&m).unwrap())
            .collect();

        let expected = vec![
            vec![vec![0], vec![2]],
            vec![vec![0], vec![3]],
            vec![vec![1], vec![2]],
            vec![vec![1], vec![3]],
        ];

        assert_eq!(combos, expected);

        let uniq: HashSet<_> = combos.iter().cloned().collect();
        assert_eq!(uniq.len(), expected.len());
    }

    #[test]
    fn iter_unwrapped_empty_slot() {
        let mm = MaskedMatching::from_matching_ref(&[vec![0], vec![]]);
        let mut it = mm.iter_unwrapped();
        assert_eq!(it.next(), None);
    }

    #[test]
    fn iter_unwrapped_no_slots() {
        let mm = MaskedMatching::from_masks(Vec::new());
        let mut it = mm.iter_unwrapped();
        assert_eq!(it.next(), None);
    }

    #[test]
    fn iter_unwrapped_single_slot_multiple_bits() {
        // One slot containing three values - three combinations total.
        let mm = MaskedMatching::from_matching_ref(&[vec![5, 7, 9]]);

        let combos: Vec<Vec<Vec<IdBase>>> = mm
            .iter_unwrapped()
            .map(|m| Vec::try_from(&m).unwrap())
            .collect();

        let expected = vec![vec![vec![5]], vec![vec![7]], vec![vec![9]]];

        assert_eq!(combos, expected);
    }

    #[test]
    fn iter_pairs_highest_bit() {
        // Build a Bitset that contains only the highest bit.
        let highest = WORD_BITS as u8 - 1; // e.g., 63 for a 64-bit word
        let mm = MaskedMatching::from_matching_ref(&[vec![highest]]);

        let pairs: Vec<(IdBase, IdBase)> = mm.iter_pairs().collect();

        assert_eq!(pairs, vec![(0, highest as IdBase)]);
    }
}
