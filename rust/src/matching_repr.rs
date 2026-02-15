pub mod bitset;
mod iter;
mod conversions;

use serde::{Deserialize, Serialize};

use crate::matching_repr::bitset::Bitset;

pub type Word = u64;
const WORD_BITS: usize = 64;
type IdBase = u8;
// const WORD_BITS_LOG: usize = 6; // log2(64)

/// Private internal representation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum MaskRepr {
    Single(Vec<Bitset>), // masks[slot]
}

/// Public type used by hot code.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MaskedMatching {
    repr: MaskRepr,
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
        match &self.repr {
            MaskRepr::Single(items) => items.get(slot).map(|b| b.is_empty()).unwrap_or(true),
        }
    }

    /// Return the bitset for `slot`.
    pub fn slot_mask(&self, slot: usize) -> Option<&Bitset> {
        match &self.repr {
            MaskRepr::Single(items) => items.get(slot)
        }
    }

    /// Prepare a `Vec<Vec<IdBase>>` representation for human-friendly debugging.
    ///
    /// Each slot becomes a `Vec<IdBase>` of the element indices present in that slot.
    pub fn prepare_debug_print(&self) -> Vec<Vec<IdBase>> {
        match &self.repr {
            MaskRepr::Single(items) => items.iter().map(|b| b.iter().collect()).collect(),
        }
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
        match &self.repr {
            MaskRepr::Single(items) => items
                .iter()
                .enumerate()
                .map(|(a, b)| {
                    let i = map_a[a as usize].clone();
                    let j = map_b[b.iter().next().unwrap() as usize].clone();
                    (i, j)
                })
                .collect(),
        }
    }

    // TODO: rename for better name
    /// Returns `true` if any slot contains at least one bit from `mask`.
    pub fn contains_mask(&self, mask: Bitset) -> bool {
        match &self.repr {
            MaskRepr::Single(items) => items.iter().any(|b| !(*b & mask).is_empty()),
        }
    }

    /// Counts the number of slots that are non-empty and overlap with `sol`.
    ///
    /// This method compares the calling `MaskedMatching` to `sol` and returns
    /// how many slots have a non-zero intersection.
    pub fn calculate_lights(&self, sol: &MaskedMatching) -> IdBase {
        match (&self.repr, &sol.repr) {
            (MaskRepr::Single(items), MaskRepr::Single(sol)) => {
                let mut l: IdBase = 0;
                for (i, j) in items.iter().enumerate() {
                    if j.is_empty() {
                        continue;
                    }
                    if !((sol[i] & *j).is_empty()) {
                        l += 1;
                    }
                }
                l
            },
        }
    }

    /// Number of slots.
    pub fn len(&self) -> usize {
        match &self.repr {
            MaskRepr::Single(m) => m.len(),
        }
    }

    /// Compute the universe (highest set bit + 1) or 0 if empty.
    ///
    /// Useful when converting back to `Vec<Vec<IdBase>>` to know the upper bound.
    pub fn computed_universe(&self) -> usize {
        match &self.repr {
            MaskRepr::Single(items) => {
                items
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
    }
}

impl<'a> std::ops::BitAnd<&'a MaskedMatching> for MaskedMatching {
    type Output = Self;

    fn bitand(mut self, rhs: &'a MaskedMatching) -> Self::Output {
        match (&mut self.repr, &rhs.repr) {
            (MaskRepr::Single(l), MaskRepr::Single(r)) => {
                for (i, j) in l.iter_mut().zip(r) {
                    *i &= *j;
                }
            }
        };
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn test_bitset_basic() {
        let mut b = Bitset::empty();
        assert!(b.is_empty());
        b.insert(3);
        assert!(!b.is_empty());
        assert!(b.contains(3));
        assert_eq!(b.count(), 1);
        b.insert(1);
        assert_eq!(b.count(), 2);
        let collected: Vec<IdBase> = b.iter().collect();
        assert_eq!(collected, vec![1, 3]);
    }

    #[test]
    fn test_bitset_ops_and_assign() {
        let a = Bitset::from_idxs(&[1u8, 3u8]);
        let b = Bitset::from_idxs(&[3u8, 5u8]);
        let and = a & b;
        assert_eq!(and, Bitset::from_idxs(&[3u8]));
        let or = a | b;
        assert!(or.contains(1));
        assert!(or.contains(3));
        assert!(or.contains(5));

        let mut c = a;
        c &= b;
        assert_eq!(c, Bitset::from_idxs(&[3u8]));
        let mut d = Bitset::empty();
        d |= a;
        assert_eq!(d, a);
    }

    #[test]
    fn test_iter_over_maskedmatching_yields_bitsets() {
        let legacy = vec![vec![1u8, 2u8], vec![0u8]];
        let mm = MaskedMatching::from(&legacy);
        let masks: Vec<Bitset> = mm.iter().collect();
        assert_eq!(masks[0], Bitset::from_idxs(&[1u8, 2u8]));
        assert_eq!(masks[1], Bitset::from_idxs(&[0u8]));
    }

    #[test]
    fn test_contains_mask_and_slot_contains_any() {
        let legacy = vec![vec![1u8, 4u8], vec![2u8]];
        let mm = MaskedMatching::from(&legacy);
        let m = Bitset::from_idxs(&[4u8]);
        assert!(mm.contains_mask(m));
        let mut set = HashSet::new();
        set.insert(3u8);
        set.insert(4u8);
        assert!(mm.slot_mask(0).unwrap().contains_any_idx(&set));
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
    fn test_tryfrom_hashmap_and_roundtrip_conversion() {
        let mut map = HashMap::new();
        map.insert(0u8, 7u8);
        map.insert(2u8, 3u8);
        let mm = MaskedMatching::try_from(map).unwrap();
        let as_vec = Vec::try_from(&mm).unwrap();
        assert_eq!(as_vec[0], vec![7u8]);
        assert_eq!(as_vec[2], vec![3u8]);

        // roundtrip via from_masks
        let masks = mm.iter().collect::<Vec<_>>();
        let mm2 = MaskedMatching::from_masks(masks);
        let round: Vec<Vec<IdBase>> = Vec::try_from(&mm2).unwrap();
        assert_eq!(round, as_vec);
    }

    #[test]
    fn test_prepare_debug_print() {
        let legacy = vec![vec![3u8, 5u8], vec![0u8]];
        let mm = MaskedMatching::from(&legacy);
        let dbg = mm.prepare_debug_print();
        assert_eq!(dbg, legacy);
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
    fn test_tryfrom_hashmap() {
        let mut map = HashMap::new();
        map.insert(0u8, 7u8);
        map.insert(2u8, 3u8);
        let mm = MaskedMatching::try_from(map).unwrap();
        let as_vec = Vec::try_from(&mm).unwrap();
        assert_eq!(as_vec[0], vec![7u8]);
        assert_eq!(as_vec[2], vec![3u8]);
    }

    #[test]
    fn test_bitset_binary_fmt_and_ord() {
        let a = Bitset::from_idxs(&[0u8, 2u8]); // 1 + 4 = 5 -> "101"
        assert_eq!(format!("{:b}", a), "101");

        // Ord/PartialOrd are derived over the inner word; check numeric ordering.
        let small = Bitset::from_idxs(&[1u8]); // 2
        let large = Bitset::from_idxs(&[3u8]); // 8
        assert!(small < large);
        assert_eq!(small.cmp(&large), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_from_word_and_from_masks_roundtrip() {
        let masks = vec![Bitset::from_word(0b101), Bitset::from_word(0b10)];
        let mm = MaskedMatching::from_masks(masks.clone());
        let collected: Vec<Bitset> = mm.iter().collect();
        assert_eq!(collected, masks);

        // Roundtrip to Vec<Vec<IdBase>> and back
        let as_vec: Vec<Vec<IdBase>> = Vec::try_from(&mm).unwrap();
        let mm2 = MaskedMatching::from(&as_vec);
        let collected2: Vec<Bitset> = mm2.iter().collect();
        assert_eq!(collected2, masks);
    }



    // ---- Bitset tests ----

    #[test]
    fn bitset_empty_and_from_word_and_from_idxs() {
        let e = Bitset::empty();
        assert_eq!(e.0, 0);
        let w = Bitset::from_word(0b1010);
        assert_eq!(w.as_word(), 0b1010);
        let s = Bitset::from_idxs(&[1u8, 3u8]);
        assert_eq!(s, w);
    }

    #[test]
    fn bitset_insert_and_contains_and_count_is_empty() {
        let mut b = Bitset::empty();
        assert!(b.is_empty());
        b.insert(5);
        assert!(!b.is_empty());
        assert!(b.contains(5));
        assert_eq!(b.count(), 1);
        b.insert(2);
        assert_eq!(b.count(), 2);
    }

    #[test]
    fn bitset_contains_any_idx() {
        let b = Bitset::from_idxs(&[1u8, 4u8]);
        let mut set = HashSet::new();
        set.insert(0u8);
        set.insert(4u8);
        assert!(b.contains_any_idx(&set));
        set.remove(&4u8);
        assert!(!b.contains_any_idx(&set));
    }

    #[test]
    fn bitset_trailing_clear_and_iter_and_trailing_zeros() {
        let b = Bitset::from_word(0b10110);
        // trailing zeros of 0b10110 is 1
        assert_eq!(b.trailing_zeros(), 1);
        let mut it = b.iter();
        assert_eq!(it.next(), Some(1));
        assert_eq!(it.next(), Some(2));
        assert_eq!(it.next(), Some(4));
        assert_eq!(it.next(), None);

        // clear_lowest_bit works
        let mut b2 = Bitset::from_word(0b10110);
        b2.clear_lowest_bit();
        assert_eq!(b2.as_word(), 0b10100);
    }

    #[test]
    fn bitset_and_or_and_assign_ops() {
        let a = Bitset::from_idxs(&[1u8, 3u8]);
        let b = Bitset::from_idxs(&[3u8, 5u8]);
        assert_eq!((a & b), Bitset::from_idxs(&[3u8]));
        assert_eq!((a | b), Bitset::from_idxs(&[1u8, 3u8, 5u8]));
        let mut c = a;
        c &= b;
        assert_eq!(c, Bitset::from_idxs(&[3u8]));
        let mut d = Bitset::empty();
        d |= a;
        assert_eq!(d, a);
    }

    #[test]
    fn bitset_binary_format_and_ord() {
        let a = Bitset::from_idxs(&[0u8, 2u8]); // 5 -> "101"
        assert_eq!(format!("{:b}", a), "101");
        let small = Bitset::from_idxs(&[1u8]); // 2
        let large = Bitset::from_idxs(&[3u8]); // 8
        assert!(small < large);
    }

    // ---- MaskedMatching basic constructors + conversions ----

    #[test]
    fn maskedmatching_from_masks_and_with_slots_and_len_and_iter() {
        let masks = vec![Bitset::from_idxs(&[0]), Bitset::from_idxs(&[1,3])];
        let mm = MaskedMatching::from_masks(masks.clone());
        assert_eq!(mm.len(), 2);
        let collected: Vec<Bitset> = mm.iter().collect();
        assert_eq!(collected, masks);

        let mm2 = MaskedMatching::with_slots(3);
        assert_eq!(mm2.len(), 3);
        for (i, mask) in mm2.iter().enumerate() {
            assert!(mask.is_empty(), "slot {} should be empty", i);
        }
    }

    #[test]
    fn maskedmatching_from_matching_ref_and_computed_universe() {
        let legacy = vec![vec![0u8, 2u8], vec![1u8]];
        let mm = MaskedMatching::from_matching_ref(&legacy);
        assert_eq!(mm.computed_universe(), 3);
        let dbg = mm.prepare_debug_print();
        assert_eq!(dbg, legacy);
    }

    #[test]
    fn prepare_debug_print_names_ok_and_panic_on_bad_maps() {
        let legacy = vec![vec![0u8], vec![1u8]];
        let mm = MaskedMatching::from(&legacy);
        let map_a = vec!["a".to_string(), "b".to_string()];
        let map_b = vec!["x".to_string(), "y".to_string()];
        let res = mm.prepare_debug_print_names(&map_a, &map_b);
        assert_eq!(res, vec![("a".to_string(), "x".to_string()), ("b".to_string(), "y".to_string())]);

        // If map_b too small the function is expected to panic; we explicitly assert that.
        let map_b_small = vec!["only".to_string()];
        let result = std::panic::catch_unwind(|| mm.prepare_debug_print_names(&map_a, &map_b_small));
        assert!(result.is_err());
    }

    #[test]
    fn contains_mask_and_slot_contains_any() {
        let legacy = vec![vec![1u8, 4u8], vec![2u8]];
        let mm = MaskedMatching::from(&legacy);
        let m = Bitset::from_idxs(&[4u8]);
        assert!(mm.contains_mask(m));
        let mut set = HashSet::new();
        set.insert(3u8);
        set.insert(4u8);
        assert!(mm.slot_mask(0).contains_any_idx(&set));
    }

    #[test]
    fn calculate_lights_counts_overlaps() {
        let mm = MaskedMatching::from(&vec![vec![0u8], vec![1u8], vec![2u8]]);
        let mm2 = MaskedMatching::from(&vec![vec![0u8], vec![9u8], vec![2u8]]);
        assert_eq!(mm.calculate_lights(&mm2), 2u8); // slots 0 and 2 overlap
    }

    #[test]
    fn try_from_maskedmatching_to_vec_roundtrip() {
        let legacy = vec![vec![1u8], vec![2u8,3u8]];
        let mm = MaskedMatching::from(&legacy);
        let back: Vec<Vec<IdBase>> = Vec::try_from(&mm).expect("conversion back failed");
        assert_eq!(back, legacy);
    }

    #[test]
    fn try_from_hashmap_to_maskedmatching_and_roundtrip() {
        let mut map = HashMap::new();
        map.insert(0u8, 7u8);
        map.insert(2u8, 3u8);
        let mm = MaskedMatching::try_from(map).unwrap();
        let as_vec = Vec::try_from(&mm).unwrap();
        assert_eq!(as_vec[0], vec![7u8]);
        assert_eq!(as_vec[2], vec![3u8]);
    }

    #[test]
    fn maskedmatching_bitand_operator() {
        let a = MaskedMatching::from(&vec![vec![0u8, 1u8], vec![2u8]]);
        let b = MaskedMatching::from(&vec![vec![1u8], vec![2u8]]);
        let c = a.clone() & &b;
        // c slot 0 should be intersection {1}, slot1 {2}
        let c_vec: Vec<Vec<IdBase>> = Vec::try_from(&c).unwrap();
        assert_eq!(c_vec[0], vec![1u8]);
        assert_eq!(c_vec[1], vec![2u8]);
    }

    // ---- Iterators tests ----

    #[test]
    fn pairs_iter_yields_expected_pairs() {
        let mm = MaskedMatching::from(&vec![vec![1u8, 2u8], vec![0u8]]);
        let pairs: Vec<(IdBase, IdBase)> = mm.iter_pairs().collect();
        assert_eq!(pairs, vec![(0,1), (0,2), (1,0)]);
    }

    #[test]
    fn slots_iter_returns_bitsets() {
        let legacy = vec![vec![1u8], vec![0u8, 3u8]];
        let mm = MaskedMatching::from(&legacy);
        let masks: Vec<Bitset> = mm.iter().collect();
        assert_eq!(masks, vec![Bitset::from_idxs(&[1u8]), Bitset::from_idxs(&[0u8,3u8])]);
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
        let combos: Vec<Vec<Vec<IdBase>>> = mm.iter_unwrapped().map(|m| Vec::try_from(&m).unwrap()).collect();

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

    // ---- conversion roundtrip & extras ----

    #[test]
    fn from_masks_roundtrip_via_tryfrom() {
        let masks = vec![Bitset::from_word(0b101), Bitset::from_word(0b10)];
        let mm = MaskedMatching::from_masks(masks.clone());
        let collected: Vec<Bitset> = mm.iter().collect();
        assert_eq!(collected, masks);

        let as_vec: Vec<Vec<IdBase>> = Vec::try_from(&mm).unwrap();
        let mm2 = MaskedMatching::from(&as_vec);
        let collected2: Vec<Bitset> = mm2.iter().collect();
        assert_eq!(collected2, masks);
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
