use core::fmt;
use std::collections::HashMap;

use crate::matching_repr::{bitset::Bitset, IdBase, MaskedMatching, Word};

#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    UniverseTooLarge(usize, usize),
    RequiredSlotsNotFound,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConversionError::UniverseTooLarge(universe, max) => {
                write!(
                    f,
                    "universe ({}) too large to convert back to Matching (max {})",
                    universe, max
                )
            }
            ConversionError::RequiredSlotsNotFound => {
                write!(f, "Could not obtain the number of slots required",)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

impl MaskedMatching {
    /// Consume self and return the owned `Vec<Bitset>` masks.
    /// Use for zero-copy handoff: move the internal vector out, then re-use it.
    pub fn into_masks(self) -> Vec<Bitset> {
        self.masks
    }

    /// Construct from raw bitset masks.
    pub fn from_masks(masks: Vec<Bitset>) -> Self {
        MaskedMatching { masks }
    }

    /// Swap the internal Vec<Bitset> with `other`.
    /// This provides zero-copy handoff of mask storage to/from the MaskedMatching.
    /// After calling `self.swap_masks(&mut buf)`, `self` will own the contents of
    /// `buf` and `buf` will own what `self` used to own.
    #[inline]
    pub fn swap_masks(&mut self, other: &mut Vec<Bitset>) {
        std::mem::swap(&mut self.masks, other)
    }

    /// Create empty with `slots`.
    pub fn with_slots(slots: usize) -> Self {
        MaskedMatching {
            masks: vec![Bitset::empty(); slots],
        }
    }

    /// Replace the internal masks with the provided slice *without* allocating on every call,
    /// provided the internal Vec already has enough capacity.
    ///
    /// This method performs a *copy* of `slice` elements (cheap, u64-sized), but it avoids
    /// heap allocations if the internal Vec capacity >= slice.len().
    #[inline]
    pub fn set_masks_from_slice(&mut self, slice: &[Bitset]) {
        // If capacity is insufficient, reserve once (might allocate once).
        if self.masks.capacity() < slice.len() {
            self.masks.reserve(slice.len() - self.masks.capacity());
        }
        // Fast path: if we can, overwrite existing elements and adjust length.
        // We'll use safe APIs: clear + extend_from_slice. Because we've reserved,
        // extend_from_slice will not allocate in the hot path.
        self.masks.clear();
        self.masks.extend_from_slice(slice);
    }

    /// Construct from legacy `Matching` reference.
    ///
    /// The `m` is expected to be a `Vec` of slots; each slot is a `Vec<IdBase>`
    /// listing value indices.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ayto::matching_repr::MaskedMatching;
    /// let m = vec![vec![0, 2], vec![1]];
    /// let mm = MaskedMatching::from_matching_ref(&m);
    /// assert_eq!(mm.computed_universe(), 3);
    /// ```
    pub fn from_matching_ref(m: &[Vec<IdBase>]) -> Self {
        let mut masks: Vec<Bitset> = Vec::with_capacity(m.len());
        for slot in m.iter() {
            // construct the value representing the vector 'slot'
            let mut w: Word = 0;
            for &v in slot.iter() {
                let idx = v as usize;
                // safe because we assume inputs fit in WORD_BITS
                w |= (1 as Word) << idx;
            }
            masks.push(Bitset::from_word(w));
        }
        MaskedMatching { masks }
    }
}

/// From<&Vec<Vec<IdBase>>> and Vec
impl From<&Vec<Vec<IdBase>>> for MaskedMatching {
    fn from(m: &Vec<Vec<IdBase>>) -> Self {
        MaskedMatching::from_matching_ref(m)
    }
}
impl From<Vec<Vec<IdBase>>> for MaskedMatching {
    fn from(m: Vec<Vec<IdBase>>) -> Self {
        MaskedMatching::from_matching_ref(&m)
    }
}

impl From<&[IdBase]> for MaskedMatching {
    fn from(ms: &[IdBase]) -> Self {
        let mut slots = vec![];
        for m in ms {
            let mut b = Bitset::empty();
            b.insert(*m);
            slots.push(b);
        }
        MaskedMatching::from_masks(slots)
    }
}

impl From<(IdBase, IdBase)> for MaskedMatching {
    fn from(m: (IdBase, IdBase)) -> Self {
        let mut slots = vec![Bitset::empty(); m.0 as usize + 1];
        slots[m.0 as usize].insert(m.1);
        MaskedMatching::from_masks(slots)
    }
}

/// TryFrom back to Vec<Vec<IdBase>> (errors if universe > IdBase::MAX+1)
impl TryFrom<&MaskedMatching> for Vec<Vec<IdBase>> {
    type Error = ConversionError;
    fn try_from(masked: &MaskedMatching) -> Result<Self, Self::Error> {
        let universe = masked.computed_universe();
        if universe > (IdBase::MAX as usize) + 1 {
            return Err(ConversionError::UniverseTooLarge(
                universe,
                IdBase::MAX as usize,
            ));
        }
        let mut out: Vec<Vec<IdBase>> = Vec::with_capacity(masked.len());
        for b in masked.masks.iter() {
            let mut slot = Vec::new();
            for i in b.iter() {
                slot.push(i);
            }
            out.push(slot);
        }
        Ok(out)
    }
}

impl TryFrom<HashMap<IdBase, IdBase>> for MaskedMatching {
    type Error = ConversionError;
    fn try_from(masked: HashMap<IdBase, IdBase>) -> Result<Self, Self::Error> {
        let max = *masked
            .keys()
            .max()
            .ok_or(ConversionError::RequiredSlotsNotFound)?;
        let mut slots = vec![Bitset::empty(); (max as usize) + 1];
        for (k, v) in masked.into_iter() {
            slots[k as usize].insert(v);
        }
        Ok(MaskedMatching::from_masks(slots))
    }
}

impl From<Vec<Bitset>> for MaskedMatching {
    fn from(m: Vec<Bitset>) -> Self {
        MaskedMatching { masks: m }
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

    #[test]
    fn maskedmatching_from_masks_and_with_slots_and_len_and_iter() {
        let masks = vec![Bitset::from_idxs(&[0]), Bitset::from_idxs(&[1, 3])];
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
        assert_eq!(
            res,
            vec![
                ("a".to_string(), "x".to_string()),
                ("b".to_string(), "y".to_string())
            ]
        );

        // If map_b too small the function is expected to panic; we explicitly assert that.
        let map_b_small = vec!["only".to_string()];
        let result =
            std::panic::catch_unwind(|| mm.prepare_debug_print_names(&map_a, &map_b_small));
        assert!(result.is_err());
    }

    #[test]
    fn contains_mask_and_slot_contains_any() {
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
    fn calculate_lights_counts_overlaps() {
        let mm = MaskedMatching::from(&vec![vec![0u8], vec![1u8], vec![2u8]]);
        let mm2 = MaskedMatching::from(&vec![vec![0u8], vec![9u8], vec![2u8]]);
        assert_eq!(mm.calculate_lights(&mm2), 2u8); // slots 0 and 2 overlap
    }

    #[test]
    fn try_from_maskedmatching_to_vec_roundtrip() {
        let legacy = vec![vec![1u8], vec![2u8, 3u8]];
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
