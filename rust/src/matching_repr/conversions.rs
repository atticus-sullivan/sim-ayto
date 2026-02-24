/// This module contains all kinds of conversions from and to MaskedMatching.
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
        self.masks.extend(slice);
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

impl From<Vec<Bitset>> for MaskedMatching {
    fn from(m: Vec<Bitset>) -> Self {
        MaskedMatching { masks: m }
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn from_masks_simple() {
        let masks = vec![Bitset::from_word(0b101), Bitset::from_word(0b10)];
        let mm = MaskedMatching::from_masks(masks.clone());
        assert_eq!(mm.masks, masks);
    }
    #[test]
    fn into_masks_simple() {
        let original_masks = vec![Bitset::from_word(0b1010), Bitset::from_word(0b0101)];
        let mm = MaskedMatching::from_masks(original_masks.clone());
        let moved = mm.into_masks();
        assert_eq!(moved, original_masks);
    }

    #[test]
    fn swap_masks_simple() {
        let mut mm =
            MaskedMatching::from_masks(vec![Bitset::from_word(0b101), Bitset::from_word(0b010)]);
        let mut external = vec![Bitset::from_word(0b111)];

        mm.swap_masks(&mut external);

        assert_eq!(mm.masks, vec![Bitset::from_word(0b111)]);
        assert_eq!(
            external,
            vec![Bitset::from_word(0b101), Bitset::from_word(0b010),]
        );
    }

    #[test]
    fn with_slots_simple() {
        let mm2 = MaskedMatching::with_slots(3);
        assert_eq!(mm2.len(), 3);
        for (i, mask) in mm2.iter().enumerate() {
            assert!(mask.is_empty(), "slot {} should be empty", i);
        }
    }

    #[test]
    fn set_masks_from_slice_cap_enough() {
        // Path where capacity is already sufficient.
        let mut mm = MaskedMatching::with_slots(5);
        let slice = [
            Bitset::from_word(0b001),
            Bitset::from_word(0b010),
            Bitset::from_word(0b100),
        ];
        mm.set_masks_from_slice(&slice);
        assert_eq!(mm.masks, slice);
    }

    #[test]
    fn set_masks_from_slice_cap_too_small() {
        // Path where we need to reserve additional capacity.
        let mut mm = MaskedMatching::with_slots(0);
        let long_slice = [
            Bitset::from_word(0b1),
            Bitset::from_word(0b10),
            Bitset::from_word(0b100),
            Bitset::from_word(0b1000),
        ];

        let prev_cap = mm.masks.capacity();
        mm.set_masks_from_slice(&long_slice);

        assert_eq!(mm.masks, long_slice);
        assert!(mm.masks.capacity() >= long_slice.len());
        assert!(mm.masks.capacity() >= prev_cap);
    }

    #[test]
    fn from_matching_ref_and_into_vector() -> Result<()> {
        let legacy = vec![vec![1u8], vec![2u8, 3u8]];
        let mm = MaskedMatching::from_matching_ref(&legacy);
        assert_eq!(
            mm.masks,
            vec![Bitset::from_idxs(&[1]), Bitset::from_idxs(&[2, 3]),]
        );
        let back: Vec<Vec<IdBase>> = Vec::try_from(&mm)?;
        assert_eq!(back, legacy);
        Ok(())
    }

    // [IdBase] -> MaskedMatching
    #[test]
    fn from_idbase_slice() {
        let mm = MaskedMatching::from(&[1 as IdBase, 2 as IdBase][..]);
        assert_eq!(mm.masks, vec![Bitset::from_idxs(&[1, 2]),])
    }

    // (idBase, idBase) -> MaskedMatching
    #[test]
    fn from_idbase_tuple() {
        let mm = MaskedMatching::from((1 as IdBase, 2 as IdBase));
        assert_eq!(
            mm.masks,
            vec![Bitset::from_idxs(&[]), Bitset::from_idxs(&[2]),]
        )
    }

    // Vec<Bitset> -> MaskedMatching
    #[test]
    fn from_bitset_vector() {
        let mm = MaskedMatching::from(vec![Bitset::from_idxs(&[1, 2]), Bitset::from_idxs(&[3, 4])]);
        assert_eq!(
            mm.masks,
            vec![Bitset::from_idxs(&[1, 2]), Bitset::from_idxs(&[3, 4]),]
        )
    }

    // HashMap<IdBase, IdBase> -> MaskedMatching (tryfrom)
    #[test]
    fn try_from_idbase_hashmap() -> Result<()> {
        let mm = MaskedMatching::try_from(HashMap::from_iter([(0, 7), (2, 3)]))?;
        assert_eq!(
            mm.masks,
            vec![
                Bitset::from_idxs(&[7]),
                Bitset::from_idxs(&[]),
                Bitset::from_idxs(&[3]),
            ]
        );
        Ok(())
    }
    #[test]
    fn try_from_idbase_hashmap_conversion_error() {
        let empty_map: HashMap<IdBase, IdBase> = HashMap::new();
        let err = MaskedMatching::try_from(empty_map).unwrap_err();
        match err {
            ConversionError::RequiredSlotsNotFound => {}
            _ => panic!("expected RequiredSlotsNotFound error"),
        }
    }

    // MaskedMatching -> VecVecIdBase (tryfrom)
    #[test]
    fn try_to_matching_ref() -> Result<()> {
        let a = vec![vec![1, 2], vec![], vec![3, 4]];
        let b = Vec::<Vec<IdBase>>::try_from(&MaskedMatching::from_matching_ref(&a))?;
        assert_eq!(b, a);
        Ok(())
    }
}
