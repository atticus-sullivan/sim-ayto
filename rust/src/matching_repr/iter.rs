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

    /// Iterate over combinations that pick exactly one value from each slot,
    /// producing `MaskedMatching` objects that have single-bit masks per slot.
    pub fn iter_unwrapped(&self) -> UnwrappedIter<'_> {
        // Build the actual iterators and *consume* their first element to
        // populate `current`. This keeps `iters` and `current` in sync.
        let mut iters = self.masks.iter().map(|b| b.iter()).collect::<Vec<_>>();

        let mut current: Vec<Option<IdBase>> = Vec::with_capacity(iters.len());
        for it in iters.iter_mut() {
            // consume the first element from the actual iterator so `iters`
            // is positioned *after* the current choice
            current.push(it.next());
        }

        let done = current.iter().any(|x| x.is_none());
        UnwrappedIter {
            mm: &self,
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

        // Build single-bit mask slots from current selection
        let slots = self
            .current
            .iter()
            .map(|&opt_idx| {
                if let Some(idx) = opt_idx {
                    Bitset::from_idxs(&[idx])
                } else {
                    Bitset::empty()
                }
            })
            .collect::<Vec<Bitset>>();

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
                return Some(MaskedMatching::from_masks(slots));
            }
        }

        // exhausted: mark done and return the last combination once
        self.done = true;
        Some(MaskedMatching::from_masks(slots))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_over_maskedmatching_yields_bitsets() {
        let legacy = vec![vec![1u8, 2u8], vec![0u8]];
        let mm = MaskedMatching::from(&legacy);
        let masks: Vec<Bitset> = mm.iter().collect();
        assert_eq!(masks[0], Bitset::from_idxs(&[1u8, 2u8]));
        assert_eq!(masks[1], Bitset::from_idxs(&[0u8]));
    }

    #[test]
    fn test_iter_pairs_ordering() {
        // slot 0: {1,2}, slot1:{0}
        let legacy = vec![vec![1u8, 2u8], vec![0u8]];
        let mm = MaskedMatching::from(&legacy);
        let pairs: Vec<(IdBase, IdBase)> = mm.iter_pairs().collect();
        // order: slots increasing; values in slot increasing
        assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 0)]);
    }

    #[test]
    fn test_iter_unwrapped_cartesian_product_exact() {
        // two slots: slot0 {0,1}, slot1 {2,3}
        let legacy = vec![vec![0u8, 1u8], vec![2u8, 3u8]];
        let mm = MaskedMatching::from(&legacy);

        // collect unwrapped combinations
        let combos: Vec<Vec<Vec<IdBase>>> = mm
            .iter_unwrapped()
            .map(|m| Vec::try_from(&m).unwrap())
            .collect();

        let expected = vec![
            vec![vec![0u8], vec![2u8]],
            vec![vec![0u8], vec![3u8]],
            vec![vec![1u8], vec![2u8]],
            vec![vec![1u8], vec![3u8]],
        ];

        // order produced by our iterator should be as above
        assert_eq!(combos.len(), expected.len());
        assert_eq!(combos, expected);
    }

    #[test]
    fn test_iter_pairs() {
        // slot 0: {1,2}, slot1:{0}
        let legacy = vec![vec![1u8, 2u8], vec![0u8]];
        let mm = MaskedMatching::from(&legacy);
        let pairs: Vec<(IdBase, IdBase)> = mm.iter_pairs().collect();
        // order: slots increasing; values in slot increasing
        assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 0)]);
    }

    #[test]
    fn test_iter_unwrapped_product() {
        // two slots: slot0 {0,1}, slot1 {2,3}
        let legacy = vec![vec![0u8, 1u8], vec![2u8, 3u8]];
        let mm = MaskedMatching::from(&legacy);
        // collect unwrapped combinations
        let combos: Vec<Vec<Vec<IdBase>>> = mm
            .iter_unwrapped()
            .map(|m| {
                // represent each MaskedMatching as Vec<Vec<IdBase>>
                Vec::try_from(&m).unwrap()
            })
            .collect();
        // there are 4 combinations (2 x 2)
        assert_eq!(combos.len(), 4);
        // each combination should contain one element per slot
        for c in combos {
            assert_eq!(c.len(), 2);
            assert!(c[0].len() == 1 && c[1].len() == 1);
        }
    }

    #[test]
    fn test_iter_unwrapped_empty_slot_yields_none() {
        // slot 0 has values, slot 1 is empty -> product empty
        let legacy = vec![vec![0u8], vec![]];
        let mm = MaskedMatching::from(&legacy);
        let mut it = mm.iter_unwrapped();
        assert_eq!(it.next(), None);
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

    #[test]
    fn pairs_iter_yields_expected_pairs() {
        let mm = MaskedMatching::from(&vec![vec![1u8, 2u8], vec![0u8]]);
        let pairs: Vec<(IdBase, IdBase)> = mm.iter_pairs().collect();
        assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 0)]);
    }

    #[test]
    fn slots_iter_returns_bitsets() {
        let legacy = vec![vec![1u8], vec![0u8, 3u8]];
        let mm = MaskedMatching::from(&legacy);
        let masks: Vec<Bitset> = mm.iter().collect();
        assert_eq!(
            masks,
            vec![Bitset::from_idxs(&[1u8]), Bitset::from_idxs(&[0u8, 3u8])]
        );
    }

    #[test]
    fn bititer_clone_independent() {
        let b = Bitset::from_idxs(&[1u8, 4u8]);
        let mut it1 = b.iter();
        let mut it2 = it1.clone();
        assert_eq!(it1.next(), Some(1));
        assert_eq!(it2.next(), Some(1)); // clone should not affect original
        assert_eq!(it1.next(), Some(4));
        assert_eq!(it2.next(), Some(4));
    }

    #[test]
    fn iter_unwrapped_cartesian_product_sequence_and_unique() {
        let legacy = vec![vec![0u8, 1u8], vec![2u8, 3u8]];
        let mm = MaskedMatching::from(&legacy);
        let combos: Vec<Vec<Vec<IdBase>>> = mm
            .iter_unwrapped()
            .map(|m| Vec::try_from(&m).unwrap())
            .collect();

        let expected = vec![
            vec![vec![0u8], vec![2u8]],
            vec![vec![0u8], vec![3u8]],
            vec![vec![1u8], vec![2u8]],
            vec![vec![1u8], vec![3u8]],
        ];
        assert_eq!(combos, expected);

        // also assert uniqueness (no duplicates)
        use std::collections::HashSet as HS;
        let set: HS<_> = combos.into_iter().collect();
        assert_eq!(set.len(), expected.len());
    }

    #[test]
    fn iter_unwrapped_empty_slot_yields_none() {
        let legacy = vec![vec![0u8], vec![]];
        let mm = MaskedMatching::from(&legacy);
        assert_eq!(mm.iter_unwrapped().next(), None);
    }
}
